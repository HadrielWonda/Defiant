#include "defiant/core/app.hpp"
#include "defiant/utils/crypto.hpp"
#include "defiant/utils/format.hpp"
#include <emscripten/val.h>
#include <emscripten/fetch.h>
#include <cmath>
#include <chrono>

namespace Defiant {

DefiantApp::DefiantApp(const AppConfig& config) 
    : config(config), 
      api_client(std::make_unique<ApiClient>(config.api_url, config.api_key)),
      renderer(std::make_unique<WebGLRenderer>()),
      payment_form(nullptr),
      dashboard(nullptr) {
    
    // Initialize state
    app_state = nlohmann::json::object();
    app_state["initialized"] = false;
    app_state["theme"] = config.theme;
    app_state["currency"] = "USD";
    
    // Load saved state from localStorage
    loadState();
}

DefiantApp::~DefiantApp() {
    cleanup();
}

void DefiantApp::initialize() {
    // Initialize WebGL renderer
    renderer->initialize();
    
    // Initialize API client
    api_client->initialize();
    
    // Create UI components
    payment_form = std::make_unique<PaymentForm>(*this);
    dashboard = std::make_unique<Dashboard>(*this);
    
    // Setup event listeners
    setupEventListeners();
    
    // Connect to WebSocket for real-time updates
    connectWebSocket();
    
    // Update state
    app_state["initialized"] = true;
    saveState();
    
    // Emit initialization event
    emit("app.initialized", {{"timestamp", std::time(nullptr)}});
}

void DefiantApp::cleanup() {
    disconnectWebSocket();
    if (renderer) {
        renderer->cleanup();
    }
}

void DefiantApp::renderPaymentForm(const std::string& container_id, 
                                  const PaymentFormOptions& options) {
    if (!payment_form) {
        throw std::runtime_error("Payment form not initialized");
    }
    
    // Get container element from DOM
    emscripten::val document = emscripten::val::global("document");
    emscripten::val container = document.call<emscripten::val>("getElementById", 
                                                              emscripten::val(container_id));
    
    if (container.isUndefined() || container.isNull()) {
        throw std::runtime_error("Container not found: " + container_id);
    }
    
    // Clear container
    container.set("innerHTML", "");
    
    // Render payment form
    payment_form->render(container, options);
    
    // Add event listeners for form
    payment_form->setupEventListeners();
    
    emit("payment_form.rendered", {
        {"container", container_id},
        {"options", options}
    });
}

void DefiantApp::renderDashboard(const std::string& container_id, 
                                const DashboardFilters& filters) {
    if (!dashboard) {
        throw std::runtime_error("Dashboard not initialized");
    }
    
    emscripten::val document = emscripten::val::global("document");
    emscripten::val container = document.call<emscripten::val>("getElementById", 
                                                              emscripten::val(container_id));
    
    if (container.isUndefined() || container.isNull()) {
        throw std::runtime_error("Container not found: " + container_id);
    }
    
    // Clear and render dashboard
    container.set("innerHTML", "");
    dashboard->render(container, filters);
    
    // Load dashboard data
    dashboard->loadData(filters);
    
    emit("dashboard.rendered", {
        {"container", container_id},
        {"filters", filters}
    });
}

PaymentResponse DefiantApp::createPayment(const PaymentRequest& request) {
    // Validate request
    if (request.amount <= 0) {
        throw std::invalid_argument("Amount must be positive");
    }
    
    if (request.currency.empty()) {
        throw std::invalid_argument("Currency is required");
    }
    
    // Emit event before creating payment
    emit("payment.creating", {
        {"amount", request.amount},
        {"currency", request.currency}
    });
    
    // Make API call
    PaymentResponse response = api_client->createPayment(request);
    
    // Update local state
    app_state["last_payment"] = {
        {"id", response.id},
        {"amount", response.amount},
        {"currency", response.currency},
        {"timestamp", std::time(nullptr)}
    };
    
    saveState();
    
    // Emit event after creating payment
    emit("payment.created", {
        {"id", response.id},
        {"amount", response.amount},
        {"currency", response.currency},
        {"status", response.status}
    });
    
    return response;
}

Customer DefiantApp::getCustomer(const std::string& customer_id) {
    return api_client->getCustomer(customer_id);
}

std::vector<Payment> DefiantApp::listPayments(const PaymentListQuery& query) {
    return api_client->listPayments(query);
}

void DefiantApp::refundPayment(const std::string& payment_id, int64_t amount) {
    // Emit event before refund
    emit("payment.refunding", {
        {"payment_id", payment_id},
        {"amount", amount}
    });
    
    api_client->refundPayment(payment_id, amount);
    
    // Emit event after refund
    emit("payment.refunded", {
        {"payment_id", payment_id},
        {"amount", amount}
    });
}

void DefiantApp::subscribe(const std::string& event_type, EventCallback callback) {
    event_listeners[event_type].push_back(callback);
}

void DefiantApp::emit(const std::string& event_type, const nlohmann::json& data) {
    auto it = event_listeners.find(event_type);
    if (it != event_listeners.end()) {
        std::string data_str = data.dump();
        for (const auto& callback : it->second) {
            callback(event_type, data_str);
        }
    }
    
    // Also emit to parent window for JavaScript listeners
    try {
        emscripten::val::global("window").call<void>("dispatchEvent", 
            emscripten::val::global("CustomEvent").new_(
                emscripten::val(event_type),
                emscripten::val::object().set("detail", emscripten::val(data.dump()))
            )
        );
    } catch (...) {
        // Silently fail if window is not available
    }
}

std::string DefiantApp::formatCurrency(int64_t amount, const std::string& currency) {
    return FormatUtils::formatCurrency(amount, currency);
}

std::string DefiantApp::generateQRCode(const std::string& data, int size) {
    // Generate QR code using WebGL
    return renderer->generateQRCode(data, size);
}

void DefiantApp::connectWebSocket() {
    // Connect to WebSocket server
    std::string ws_url = config.api_url;
    if (ws_url.find("http://") == 0) {
        ws_url.replace(0, 7, "ws://");
    } else if (ws_url.find("https://") == 0) {
        ws_url.replace(0, 8, "wss://");
    }
    ws_url += "/ws";
    
    api_client->connectWebSocket(ws_url, 
        [this](const std::string& message) {
            handleWebSocketMessage(message);
        });
}

void DefiantApp::disconnectWebSocket() {
    api_client->disconnectWebSocket();
}

std::string DefiantApp::generateCryptoAddress(const std::string& currency) {
    CryptoUtils crypto;
    return crypto.generateAddress(currency, "mainnet");
}

bool DefiantApp::validateCryptoPayment(const std::string& tx_hash) {
    return api_client->validateCryptoTransaction(tx_hash);
}

void DefiantApp::setupEventListeners() {
    // Listen for DOM events
    emscripten::val document = emscripten::val::global("document");
    
    // Window resize
    emscripten::val::global("window").call<void>("addEventListener",
        emscripten::val("resize"),
        emscripten::val::module_property("onWindowResize")
    );
    
    // Online/offline events
    emscripten::val::global("window").call<void>("addEventListener",
        emscripten::val("online"),
        emscripten::val::module_property("onOnline")
    );
    
    emscripten::val::global("window").call<void>("addEventListener",
        emscripten::val("offline"),
        emscripten::val::module_property("onOffline")
    );
}

void DefiantApp::handleWebSocketMessage(const std::string& message) {
    try {
        nlohmann::json data = nlohmann::json::parse(message);
        std::string event_type = data["type"];
        
        // Process different event types
        if (event_type == "payment.created") {
            emit("websocket.payment.created", data["data"]);
        } else if (event_type == "payment.updated") {
            emit("websocket.payment.updated", data["data"]);
        } else if (event_type == "invoice.paid") {
            emit("websocket.invoice.paid", data["data"]);
        } else if (event_type == "customer.updated") {
            emit("websocket.customer.updated", data["data"]);
        }
    } catch (const nlohmann::json::exception& e) {
        // Log error but don't crash
        emit("websocket.error", {{"error", e.what()}});
    }
}

void DefiantApp::loadState() {
    try {
        emscripten::val localStorage = emscripten::val::global("localStorage");
        emscripten::val savedState = localStorage.call<emscripten::val>("getItem", 
                                                                      emscripten::val("defiant_state"));
        
        if (!savedState.isUndefined() && !savedState.isNull()) {
            std::string stateStr = savedState.as<std::string>();
            app_state = nlohmann::json::parse(stateStr);
        }
    } catch (...) {
        // If loading fails, use default state
        app_state = nlohmann::json::object();
        app_state["initialized"] = false;
    }
}

void DefiantApp::saveState() {
    try {
        emscripten::val localStorage = emscripten::val::global("localStorage");
        localStorage.call<void>("setItem", 
                               emscripten::val("defiant_state"),
                               emscripten::val(app_state.dump()));
    } catch (...) {
        // Silently fail if localStorage is not available
    }
}

void DefiantApp::clearState() {
    app_state.clear();
    app_state["initialized"] = false;
    saveState();
}

} // namespace Defiant
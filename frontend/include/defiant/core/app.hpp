#pragma once

#include <string>
#include <memory>
#include <functional>
#include <unordered_map>
#include <vector>
#include <nlohmann/json.hpp>

#include "defiant/ui/components.hpp"
#include "defiant/wasm/api_client.hpp"
#include "defiant/wasm/webgl_renderer.hpp"

namespace Defiant {

// Configuration
struct AppConfig {
    std::string api_url = "http://localhost:8080";
    std::string api_key;
    bool debug = false;
    std::string theme = "dark";
};

// Event system
using EventCallback = std::function<void(const std::string&, const std::string&)>;

class DefiantApp {
private:
    AppConfig config;
    std::unique_ptr<ApiClient> api_client;
    std::unique_ptr<WebGLRenderer> renderer;
    std::unordered_map<std::string, std::vector<EventCallback>> event_listeners;
    
    // UI Components
    std::unique_ptr<PaymentForm> payment_form;
    std::unique_ptr<Dashboard> dashboard;
    
    // State
    std::string current_user;
    nlohmann::json app_state;
    
public:
    DefiantApp(const AppConfig& config);
    ~DefiantApp();
    
    // Initialization
    void initialize();
    void cleanup();
    
    // UI Methods
    void renderPaymentForm(const std::string& container_id, const PaymentFormOptions& options);
    void renderDashboard(const std::string& container_id, const DashboardFilters& filters);
    void updateUI();
    
    // API Methods
    PaymentResponse createPayment(const PaymentRequest& request);
    Customer getCustomer(const std::string& customer_id);
    std::vector<Payment> listPayments(const PaymentListQuery& query);
    void refundPayment(const std::string& payment_id, int64_t amount);
    
    // Event System
    void subscribe(const std::string& event_type, EventCallback callback);
    void emit(const std::string& event_type, const nlohmann::json& data);
    
    // Utility Methods
    std::string formatCurrency(int64_t amount, const std::string& currency);
    std::string generateQRCode(const std::string& data, int size = 200);
    
    // WebSocket
    void connectWebSocket();
    void disconnectWebSocket();
    
    // Crypto
    std::string generateCryptoAddress(const std::string& currency);
    bool validateCryptoPayment(const std::string& tx_hash);
    
private:
    void setupEventListeners();
    void handleWebSocketMessage(const std::string& message);
    void updateAnimations(double delta_time);
    
    // Internal state management
    void loadState();
    void saveState();
    void clearState();
};

} // namespace Defiant
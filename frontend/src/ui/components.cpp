#include "defiant/ui/components.hpp"
#include "defiant/wasm/webgl_renderer.hpp"
#include <emscripten/val.h>
#include <emscripten/bind.h>
#include <random>
#include <sstream>
#include <iomanip>

namespace Defiant {

// Component base class
Component::Component(const std::string& id) 
    : id(id.empty() ? generateId() : id) {
    animation.enabled = true;
    animation.duration = 0.3;
    animation.easing = "cubic-bezier(0.4, 0, 0.2, 1)";
}

Component::~Component() {
    destroy();
}

void Component::destroy() {
    if (!element.isUndefined() && !element.isNull()) {
        try {
            emscripten::val parent = element["parentNode"];
            if (!parent.isUndefined() && !parent.isNull()) {
                parent.call<void>("removeChild", element);
            }
        } catch (...) {
            // Silently fail
        }
    }
}

void Component::render(emscripten::val parent) {
    // Base implementation does nothing
}

void Component::update() {
    // Base implementation does nothing
}

void Component::show() {
    if (!visible) {
        visible = true;
        setStyle("display", "");
        fadeIn();
    }
}

void Component::hide() {
    if (visible) {
        visible = false;
        fadeOut();
        emscripten::val::global("setTimeout")(emscripten::val::module_property("setDisplayNone"),
                                            emscripten::val(animation.duration * 1000),
                                            element);
    }
}

void Component::toggle() {
    if (visible) {
        hide();
    } else {
        show();
    }
}

void Component::fadeIn(double duration) {
    if (!animation.enabled) return;
    
    setStyle("opacity", "0");
    setStyle("transition", 
             "opacity " + std::to_string(duration) + "s " + animation.easing);
    
    emscripten::val::global("setTimeout")(emscripten::val::module_property("setOpacity"),
                                        emscripten::val(10),
                                        element,
                                        emscripten::val(1.0));
}

void Component::fadeOut(double duration) {
    if (!animation.enabled) return;
    
    setStyle("transition", 
             "opacity " + std::to_string(duration) + "s " + animation.easing);
    setStyle("opacity", "0");
}

void Component::slideIn(const std::string& direction, double duration) {
    if (!animation.enabled) return;
    
    std::string transform;
    if (direction == "up") transform = "translateY(20px)";
    else if (direction == "down") transform = "translateY(-20px)";
    else if (direction == "left") transform = "translateX(20px)";
    else if (direction == "right") transform = "translateX(-20px)";
    else transform = "translateY(20px)";
    
    setStyle("transform", transform);
    setStyle("opacity", "0");
    setStyle("transition", 
             "transform " + std::to_string(duration) + "s " + animation.easing + ", " +
             "opacity " + std::to_string(duration) + "s " + animation.easing);
    
    emscripten::val::global("setTimeout")(emscripten::val::module_property("resetTransform"),
                                        emscripten::val(10),
                                        element);
    emscripten::val::global("setTimeout")(emscripten::val::module_property("setOpacity"),
                                        emscripten::val(10),
                                        element,
                                        emscripten::val(1.0));
}

void Component::slideOut(const std::string& direction, double duration) {
    if (!animation.enabled) return;
    
    std::string transform;
    if (direction == "up") transform = "translateY(-20px)";
    else if (direction == "down") transform = "translateY(20px)";
    else if (direction == "left") transform = "translateX(-20px)";
    else if (direction == "right") transform = "translateX(20px)";
    else transform = "translateY(-20px)";
    
    setStyle("transition", 
             "transform " + std::to_string(duration) + "s " + animation.easing + ", " +
             "opacity " + std::to_string(duration) + "s " + animation.easing);
    setStyle("transform", transform);
    setStyle("opacity", "0");
}

void Component::addClass(const std::string& className) {
    if (!element.isUndefined() && !element.isNull()) {
        element["classList"].call<void>("add", emscripten::val(className));
    }
}

void Component::removeClass(const std::string& className) {
    if (!element.isUndefined() && !element.isNull()) {
        element["classList"].call<void>("remove", emscripten::val(className));
    }
}

void Component::setStyle(const std::string& property, const std::string& value) {
    if (!element.isUndefined() && !element.isNull()) {
        element["style"].call<void>("setProperty", 
                                   emscripten::val(property),
                                   emscripten::val(value));
    }
}

void Component::setAttribute(const std::string& name, const std::string& value) {
    if (!element.isUndefined() && !element.isNull()) {
        element.call<void>("setAttribute", 
                          emscripten::val(name),
                          emscripten::val(value));
    }
}

void Component::addEventListener(const std::string& event, emscripten::val callback) {
    if (!element.isUndefined() && !element.isNull()) {
        element.call<void>("addEventListener", 
                          emscripten::val(event),
                          callback);
    }
}

void Component::removeEventListener(const std::string& event, emscripten::val callback) {
    if (!element.isUndefined() && !element.isNull()) {
        element.call<void>("removeEventListener", 
                          emscripten::val(event),
                          callback);
    }
}

void Component::createElement(const std::string& tag) {
    emscripten::val document = emscripten::val::global("document");
    element = document.call<emscripten::val>("createElement", emscripten::val(tag));
    element.call<void>("setAttribute", emscripten::val("id"), emscripten::val(id));
}

void Component::setInnerHTML(const std::string& html) {
    if (!element.isUndefined() && !element.isNull()) {
        element.set("innerHTML", emscripten::val(html));
    }
}

std::string Component::generateId() {
    std::random_device rd;
    std::mt19937 gen(rd());
    std::uniform_int_distribution<> dis(0, 35);
    const char* chars = "abcdefghijklmnopqrstuvwxyz0123456789";
    
    std::string id = "comp_";
    for (int i = 0; i < 8; ++i) {
        id += chars[dis(gen)];
    }
    return id;
}

// Button component
Button::Button(const std::string& text, 
               const std::function<void()>& onClick,
               const std::string& id)
    : Component(id), text(text), onClick(onClick) {}

void Button::render(emscripten::val parent) {
    createElement("button");
    
    // Set base styles
    setInnerHTML(text);
    addClass("defiant-button");
    addClass("defiant-button-" + variant);
    addClass("defiant-button-" + size);
    
    if (disabled) {
        addClass("defiant-button-disabled");
        setAttribute("disabled", "true");
    }
    
    if (loading) {
        addClass("defiant-button-loading");
        setInnerHTML("<span class='defiant-button-spinner'></span>" + text);
    }
    
    // Add click handler
    if (onClick) {
        auto callback = emscripten::val::module_property("createButtonCallback");
        addEventListener("click", callback);
    }
    
    // Append to parent
    parent.call<void>("appendChild", element);
}

void Button::update() {
    if (!element.isUndefined() && !element.isNull()) {
        setInnerHTML(text);
        
        // Update classes
        std::vector<std::string> variants = {"primary", "secondary", "outline", "danger"};
        for (const auto& v : variants) {
            if (v != variant) {
                removeClass("defiant-button-" + v);
            }
        }
        addClass("defiant-button-" + variant);
        
        // Update size
        std::vector<std::string> sizes = {"small", "medium", "large"};
        for (const auto& s : sizes) {
            if (s != size) {
                removeClass("defiant-button-" + s);
            }
        }
        addClass("defiant-button-" + size);
        
        // Update disabled state
        if (disabled) {
            addClass("defiant-button-disabled");
            setAttribute("disabled", "true");
        } else {
            removeClass("defiant-button-disabled");
            element.call<void>("removeAttribute", emscripten::val("disabled"));
        }
        
        // Update loading state
        if (loading) {
            addClass("defiant-button-loading");
            setInnerHTML("<span class='defiant-button-spinner'></span>" + text);
        } else {
            removeClass("defiant-button-loading");
            setInnerHTML(text);
        }
    }
}

void Button::setText(const std::string& newText) {
    text = newText;
    update();
}

void Button::setVariant(const std::string& newVariant) {
    variant = newVariant;
    update();
}

void Button::setSize(const std::string& newSize) {
    size = newSize;
    update();
}

void Button::setDisabled(bool isDisabled) {
    disabled = isDisabled;
    update();
}

void Button::setLoading(bool isLoading) {
    loading = isLoading;
    update();
}

// Input component
Input::Input(const std::string& type, const std::string& id)
    : Component(id), type(type) {}

void Input::render(emscripten::val parent) {
    createElement("div");
    addClass("defiant-input-container");
    
    // Create label if provided
    if (!label.empty()) {
        emscripten::val labelEl = emscripten::val::global("document")
            .call<emscripten::val>("createElement", emscripten::val("label"));
        labelEl.set("innerHTML", emscripten::val(label));
        labelEl.call<void>("setAttribute", emscripten::val("for"), emscripten::val(id + "_input"));
        element.call<void>("appendChild", labelEl);
    }
    
    // Create input element
    emscripten::val inputEl = emscripten::val::global("document")
        .call<emscripten::val>("createElement", emscripten::val("input"));
    inputEl.call<void>("setAttribute", emscripten::val("id"), emscripten::val(id + "_input"));
    inputEl.call<void>("setAttribute", emscripten::val("type"), emscripten::val(type));
    inputEl.call<void>("setAttribute", emscripten::val("placeholder"), emscripten::val(placeholder));
    inputEl.set("value", emscripten::val(value));
    
    if (required) {
        inputEl.call<void>("setAttribute", emscripten::val("required"), emscripten::val("true"));
    }
    
    if (disabled) {
        inputEl.call<void>("setAttribute", emscripten::val("disabled"), emscripten::val("true"));
    }
    
    // Add event listeners
    if (onChange) {
        inputEl.call<void>("addEventListener", emscripten::val("input"),
            emscripten::val::module_property("createInputChangeCallback"));
    }
    
    if (onBlur) {
        inputEl.call<void>("addEventListener", emscripten::val("blur"),
            emscripten::val::module_property("createInputBlurCallback"));
    }
    
    element.call<void>("appendChild", inputEl);
    
    // Create error message container
    emscripten::val errorEl = emscripten::val::global("document")
        .call<emscripten::val>("createElement", emscripten::val("div"));
    errorEl.addClass("defiant-input-error");
    errorEl.set("innerHTML", emscripten::val(error));
    element.call<void>("appendChild", errorEl);
    
    // Append to parent
    parent.call<void>("appendChild", element);
}

void Input::update() {
    if (!element.isUndefined() && !element.isNull()) {
        // Update input value
        emscripten::val inputEl = element.call<emscripten::val>("querySelector", 
                                                               emscripten::val("input"));
        if (!inputEl.isUndefined() && !inputEl.isNull()) {
            inputEl.set("value", emscripten::val(value));
            inputEl.call<void>("setAttribute", 
                              emscripten::val("placeholder"),
                              emscripten::val(placeholder));
            
            // Update disabled state
            if (disabled) {
                inputEl.call<void>("setAttribute", 
                                  emscripten::val("disabled"),
                                  emscripten::val("true"));
            } else {
                inputEl.call<void>("removeAttribute", emscripten::val("disabled"));
            }
            
            // Update required state
            if (required) {
                inputEl.call<void>("setAttribute",
                                  emscripten::val("required"),
                                  emscripten::val("true"));
            } else {
                inputEl.call<void>("removeAttribute", emscripten::val("required"));
            }
        }
        
        // Update label
        emscripten::val labelEl = element.call<emscripten::val>("querySelector",
                                                               emscripten::val("label"));
        if (!labelEl.isUndefined() && !labelEl.isNull()) {
            labelEl.set("innerHTML", emscripten::val(label));
        } else if (!label.empty()) {
            // Create label if it doesn't exist
            labelEl = emscripten::val::global("document")
                .call<emscripten::val>("createElement", emscripten::val("label"));
            labelEl.set("innerHTML", emscripten::val(label));
            labelEl.call<void>("setAttribute", 
                              emscripten::val("for"),
                              emscripten::val(id + "_input"));
            element.call<void>("insertBefore", labelEl, element["firstChild"]);
        }
        
        // Update error message
        emscripten::val errorEl = element.call<emscripten::val>("querySelector",
                                                               emscripten::val(".defiant-input-error"));
        if (!errorEl.isUndefined() && !errorEl.isNull()) {
            errorEl.set("innerHTML", emscripten::val(error));
            if (error.empty()) {
                errorEl.setStyle("display", "none");
            } else {
                errorEl.setStyle("display", "block");
            }
        }
    }
}

void Input::setValue(const std::string& newValue) {
    value = newValue;
    update();
}

void Input::setPlaceholder(const std::string& newPlaceholder) {
    placeholder = newPlaceholder;
    update();
}

void Input::setLabel(const std::string& newLabel) {
    label = newLabel;
    update();
}

void Input::setError(const std::string& newError) {
    error = newError;
    update();
}

void Input::setRequired(bool isRequired) {
    required = isRequired;
    update();
}

void Input::setDisabled(bool isDisabled) {
    disabled = isDisabled;
    update();
}

void Input::validate() {
    // Basic validation logic
    if (required && value.empty()) {
        error = "This field is required";
    } else if (type == "email" && !value.empty()) {
        // Simple email validation
        if (value.find('@') == std::string::npos) {
            error = "Please enter a valid email address";
        } else {
            error = "";
        }
    } else {
        error = "";
    }
    update();
}

// JavaScript callbacks
EMSCRIPTEN_BINDINGS(component_callbacks) {
    emscripten::function("setDisplayNone", emscripten::optional_override(
        [](int delay, emscripten::val element) {
            emscripten::val::global("setTimeout")(emscripten::val::global("setDisplayNoneCallback"),
                                                emscripten::val(delay),
                                                element);
        }
    ));
    
    emscripten::function("setOpacity", emscripten::optional_override(
        [](int delay, emscripten::val element, double opacity) {
            emscripten::val::global("setTimeout")(emscripten::val::global("setOpacityCallback"),
                                                emscripten::val(delay),
                                                element,
                                                emscripten::val(opacity));
        }
    ));
    
    emscripten::function("resetTransform", emscripten::optional_override(
        [](int delay, emscripten::val element) {
            emscripten::val::global("setTimeout")(emscripten::val::global("resetTransformCallback"),
                                                emscripten::val(delay),
                                                element);
        }
    ));
    
    emscripten::function("setDisplayNoneCallback", emscripten::optional_override(
        [](emscripten::val element) {
            element["style"].set("display", "none");
        }
    ));
    
    emscripten::function("setOpacityCallback", emscripten::optional_override(
        [](emscripten::val element, double opacity) {
            element["style"].set("opacity", std::to_string(opacity));
        }
    ));
    
    emscripten::function("resetTransformCallback", emscripten::optional_override(
        [](emscripten::val element) {
            element["style"].set("transform", "");
            element["style"].set("opacity", "1");
        }
    ));
}

} // namespace Defiant
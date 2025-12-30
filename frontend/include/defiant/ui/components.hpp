#pragma once

#include <string>
#include <vector>
#include <functional>
#include <memory>
#include <emscripten/val.h>
#include <nlohmann/json.hpp>

namespace Defiant {

// Color scheme
struct ColorScheme {
    std::string primary = "#6366f1";
    std::string secondary = "#8b5cf6";
    std::string success = "#10b981";
    std::string danger = "#ef4444";
    std::string warning = "#f59e0b";
    std::string info = "#3b82f6";
    std::string dark = "#1f2937";
    std::string light = "#f3f4f6";
};

// Animation settings
struct AnimationSettings {
    bool enabled = true;
    double duration = 0.3;
    std::string easing = "cubic-bezier(0.4, 0, 0.2, 1)";
};

// Base component class
class Component {
protected:
    emscripten::val element;
    std::string id;
    bool visible = true;
    AnimationSettings animation;
    
public:
    Component(const std::string& id = "");
    virtual ~Component();
    
    // Lifecycle methods
    virtual void render(emscripten::val parent) = 0;
    virtual void update() = 0;
    virtual void destroy();
    
    // Visibility
    void show();
    void hide();
    void toggle();
    bool isVisible() const { return visible; }
    
    // Animation
    void fadeIn(double duration = 0.3);
    void fadeOut(double duration = 0.3);
    void slideIn(const std::string& direction = "up", double duration = 0.3);
    void slideOut(const std::string& direction = "down", double duration = 0.3);
    
    // Styling
    void addClass(const std::string& className);
    void removeClass(const std::string& className);
    void setStyle(const std::string& property, const std::string& value);
    void setAttribute(const std::string& name, const std::string& value);
    
    // Events
    void addEventListener(const std::string& event, emscripten::val callback);
    void removeEventListener(const std::string& event, emscripten::val callback);
    
    // Getters
    emscripten::val getElement() const { return element; }
    std::string getId() const { return id; }
    
protected:
    void createElement(const std::string& tag);
    void setInnerHTML(const std::string& html);
    std::string generateId();
};

// Button component
class Button : public Component {
private:
    std::string text;
    std::function<void()> onClick;
    std::string variant = "primary"; // primary, secondary, outline, danger
    std::string size = "medium"; // small, medium, large
    bool disabled = false;
    bool loading = false;
    
public:
    Button(const std::string& text, 
           const std::function<void()>& onClick = nullptr,
           const std::string& id = "");
    
    void render(emscripten::val parent) override;
    void update() override;
    
    void setText(const std::string& newText);
    void setVariant(const std::string& newVariant);
    void setSize(const std::string& newSize);
    void setDisabled(bool isDisabled);
    void setLoading(bool isLoading);
    
    std::string getText() const { return text; }
    std::string getVariant() const { return variant; }
};

// Input component
class Input : public Component {
private:
    std::string type = "text";
    std::string value;
    std::string placeholder;
    std::string label;
    std::string error;
    bool required = false;
    bool disabled = false;
    std::function<void(const std::string&)> onChange;
    std::function<void(const std::string&)> onBlur;
    
public:
    Input(const std::string& type = "text",
          const std::string& id = "");
    
    void render(emscripten::val parent) override;
    void update() override;
    
    void setValue(const std::string& newValue);
    void setPlaceholder(const std::string& newPlaceholder);
    void setLabel(const std::string& newLabel);
    void setError(const std::string& newError);
    void setRequired(bool isRequired);
    void setDisabled(bool isDisabled);
    
    std::string getValue() const { return value; }
    bool isValid() const { return error.empty(); }
    
private:
    void validate();
};

// Card component
class Card : public Component {
private:
    std::string title;
    std::string subtitle;
    std::vector<std::shared_ptr<Component>> children;
    bool shadow = true;
    bool bordered = true;
    std::string headerAction;
    
public:
    Card(const std::string& title = "",
         const std::string& id = "");
    
    void render(emscripten::val parent) override;
    void update() override;
    
    void setTitle(const std::string& newTitle);
    void setSubtitle(const std::string& newSubtitle);
    void addChild(std::shared_ptr<Component> child);
    void clearChildren();
    void setShadow(bool hasShadow);
    void setBordered(bool hasBorder);
    void setHeaderAction(const std::string& action);
};

// Modal component
class Modal : public Component {
private:
    std::string title;
    std::vector<std::shared_ptr<Component>> children;
    bool open = false;
    std::function<void()> onClose;
    std::string size = "medium"; // small, medium, large, full
    
public:
    Modal(const std::string& title = "",
          const std::string& id = "");
    
    void render(emscripten::val parent) override;
    void update() override;
    void destroy() override;
    
    void openModal();
    void closeModal();
    void toggleModal();
    bool isOpen() const { return open; }
    
    void setTitle(const std::string& newTitle);
    void addChild(std::shared_ptr<Component> child);
    void clearChildren();
    void setSize(const std::string& newSize);
    void setOnClose(const std::function<void()>& callback);
    
private:
    void setupOverlay();
    void handleEscapeKey(emscripten::val event);
};

// Table component
class Table : public Component {
private:
    std::vector<std::string> headers;
    std::vector<std::vector<std::string>> data;
    std::vector<std::string> actions;
    bool striped = true;
    bool hoverable = true;
    bool sortable = false;
    int currentPage = 1;
    int pageSize = 10;
    int totalItems = 0;
    std::function<void(int, const std::string&)> onRowClick;
    std::function<void(int, int)> onPageChange;
    
public:
    Table(const std::vector<std::string>& headers = {},
          const std::string& id = "");
    
    void render(emscripten::val parent) override;
    void update() override;
    
    void setData(const std::vector<std::vector<std::string>>& newData);
    void setHeaders(const std::vector<std::string>& newHeaders);
    void setActions(const std::vector<std::string>& newActions);
    void setStriped(bool isStriped);
    void setHoverable(bool isHoverable);
    void setSortable(bool isSortable);
    void setPagination(int pageSize, int totalItems);
    void setOnRowClick(const std::function<void(int, const std::string&)>& callback);
    void setOnPageChange(const std::function<void(int, int)>& callback);
    
    void sort(int column, bool ascending = true);
    void goToPage(int page);
    
private:
    void renderHeader();
    void renderBody();
    void renderPagination();
    std::vector<std::vector<std::string>> getPageData();
};

// Chart component (using WebGL)
class Chart : public Component {
private:
    std::string type; // line, bar, pie, doughnut
    nlohmann::json data;
    nlohmann::json options;
    std::unique_ptr<WebGLRenderer> renderer;
    int width = 400;
    int height = 300;
    
public:
    Chart(const std::string& type = "line",
          const std::string& id = "");
    
    void render(emscripten::val parent) override;
    void update() override;
    void destroy() override;
    
    void setData(const nlohmann::json& newData);
    void setOptions(const nlohmann::json& newOptions);
    void setType(const std::string& newType);
    void setSize(int newWidth, int newHeight);
    
    void updateData(const nlohmann::json& newData);
    void animate();
    
private:
    void setupCanvas();
    void renderChart();
    void cleanupChart();
};

// Notification system
class Notification {
private:
    static std::vector<Notification> notifications;
    
    std::string id;
    std::string title;
    std::string message;
    std::string type; // success, error, warning, info
    int duration = 5000; // ms
    bool closable = true;
    
public:
    static void show(const std::string& title, 
                     const std::string& message, 
                     const std::string& type = "info");
    static void success(const std::string& title, const std::string& message);
    static void error(const std::string& title, const std::string& message);
    static void warning(const std::string& title, const std::string& message);
    static void info(const std::string& title, const std::string& message);
    static void clearAll();
    
private:
    void render();
    void remove();
};

} // namespace Defiant
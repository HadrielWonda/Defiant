cmake_minimum_required(VERSION 3.20)
project(DefiantFrontend)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Detect if building for WebAssembly
if (EMSCRIPTEN)
    message(STATUS "Building for WebAssembly with Emscripten")
    
    set(CMAKE_EXECUTABLE_SUFFIX ".html")
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fexceptions")
    set(CMAKE_CXX_FLAGS_DEBUG "${CMAKE_CXX_FLAGS_DEBUG} -O0 -g3")
    set(CMAKE_CXX_FLAGS_RELEASE "${CMAKE_CXX_FLAGS_RELEASE} -O3 -flto")
    
    # Emscripten settings
    set_target_properties(${PROJECT_NAME} PROPERTIES
        LINK_FLAGS "-s WASM=1 \
                    -s ALLOW_MEMORY_GROWTH=1 \
                    -s ASSERTIONS=1 \
                    -s DISABLE_EXCEPTION_CATCHING=0 \
                    -s EXPORTED_RUNTIME_METHODS='[\"ccall\",\"cwrap\",\"UTF8ToString\"]' \
                    -s EXPORTED_FUNCTIONS='[\"_main\",\"_malloc\",\"_free\"]' \
                    -s FETCH=1 \
                    -s USE_PTHREADS=1 \
                    -s PTHREAD_POOL_SIZE=4 \
                    -s USE_GLFW=3 \
                    -s FULL_ES3=1 \
                    -s OFFSCREEN_FRAMEBUFFER=1 \
                    -s SINGLE_FILE=0 \
                    --bind \
                    -lembind \
                    -s MODULARIZE=1 \
                    -s 'EXPORT_NAME=\"DefiantWasm\"' \
                    -lidbfs.js \
                    -s ENVIRONMENT=web,worker \
                    -s USE_ZLIB=1"
    )
else()
    message(STATUS "Building native application")
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -pthread")
endif()

# Include directories
include_directories(
    ${CMAKE_CURRENT_SOURCE_DIR}/src
    ${CMAKE_CURRENT_SOURCE_DIR}/include
    ../shared/include
)

# Source files
set(SOURCES
    src/main.cpp
    src/core/app.cpp
    src/core/state.cpp
    src/ui/components.cpp
    src/ui/payment_form.cpp
    src/ui/dashboard.cpp
    src/ui/animations.cpp
    src/wasm/api_client.cpp
    src/wasm/webgl_renderer.cpp
    src/wasm/websocket.cpp
    src/utils/crypto.cpp
    src/utils/validation.cpp
    src/utils/format.cpp
)

# Headers
set(HEADERS
    include/defiant/ui/components.hpp
    include/defiant/ui/payment_form.hpp
    include/defiant/ui/dashboard.hpp
    include/defiant/wasm/api_client.hpp
    include/defiant/wasm/webgl_renderer.hpp
    include/defiant/core/app.hpp
    include/defiant/core/state.hpp
    include/defiant/utils/crypto.hpp
)

# Create executable
add_executable(${PROJECT_NAME} ${SOURCES} ${HEADERS})

# Link libraries
if (EMSCRIPTEN)
    target_link_libraries(${PROJECT_NAME}
        -s USE_WEBGL2=1
        -s USE_SDL=2
        -s USE_SDL_IMAGE=2
        -s SDL2_IMAGE_FORMATS='[\"png\"]'
    )
else()
    find_package(OpenGL REQUIRED)
    find_package(GLFW3 REQUIRED)
    find_package(GLEW REQUIRED)
    
    target_link_libraries(${PROJECT_NAME}
        ${OPENGL_LIBRARIES}
        glfw
        GLEW::GLEW
    )
endif()

# Copy assets
if (EMSCRIPTEN)
    add_custom_command(TARGET ${PROJECT_NAME} POST_BUILD
        COMMAND ${CMAKE_COMMAND} -E copy_directory
            ${CMAKE_CURRENT_SOURCE_DIR}/web
            ${CMAKE_CURRENT_BINARY_DIR}/web
    )
endif()
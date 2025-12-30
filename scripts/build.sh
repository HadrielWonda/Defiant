#!/bin/bash

# Defiant Build Script
# Builds both backend (Rust) and frontend (WebAssembly) components

set -e  # Exit on error
set -o pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}  Building Defiant Payments Platform${NC}"
echo "======================================"

# Check prerequisites
check_prerequisites() {
    echo -e "${YELLOW}Checking prerequisites...${NC}"
    
    # Check for Rust
    if ! command -v cargo &> /dev/null; then
        echo -e "${RED}Error: Rust is not installed${NC}"
        echo "Install Rust from: https://rustup.rs/"
        exit 1
    fi
    
    # Check for Emscripten (for WebAssembly)
    if ! command -v emcc &> /dev/null; then
        echo -e "${YELLOW}Warning: Emscripten not found${NC}"
        echo "WebAssembly builds will be skipped"
        echo "Install Emscripten: https://emscripten.org/docs/getting_started/downloads.html"
        SKIP_WASM=true
    else
        SKIP_WASM=false
    fi
    
    # Check for Node.js (for build tools)
    if ! command -v node &> /dev/null; then
        echo -e "${YELLOW}Warning: Node.js not found${NC}"
        echo "Some build tools may not work"
    fi
    
    # Check for PostgreSQL (for database)
    if ! command -v psql &> /dev/null; then
        echo -e "${YELLOW}Warning: PostgreSQL not found${NC}"
        echo "Database operations will require Docker"
    fi
    
    echo -e "${GREEN}✓ Prerequisites checked${NC}"
}

# Build backend
build_backend() {
    echo -e "\n${YELLOW}Building Rust backend...${NC}"
    
    cd backend
    
    # Check for Rust dependencies
    echo "Updating dependencies..."
    cargo update
    
    # Build in release mode
    echo "Building release version..."
    cargo build --release --features "postgres"
    
    # Run tests
    echo "Running tests..."
    cargo test --release
    
    # Generate documentation
    echo "Generating documentation..."
    cargo doc --no-deps --release
    
    cd ..
    
    echo -e "${GREEN}✓ Backend built successfully${NC}"
}

# Build frontend
build_frontend() {
    if [ "$SKIP_WASM" = true ]; then
        echo -e "\n${YELLOW}Skipping WebAssembly frontend build${NC}"
        return
    fi
    
    echo -e "\n${YELLOW}Building C++/WebAssembly frontend...${NC}"
    
    cd frontend
    
    # Create build directory
    mkdir -p build
    cd build
    
    # Configure with Emscripten
    echo "Configuring CMake with Emscripten..."
    emcmake cmake .. -DCMAKE_BUILD_TYPE=Release
    
    # Build
    echo "Building WebAssembly module..."
    cmake --build . --config Release
    
    # Optimize WebAssembly
    echo "Optimizing WebAssembly..."
    wasm-opt -O3 defiant_wasm.wasm -o defiant_wasm_opt.wasm
    mv defiant_wasm_opt.wasm defiant_wasm.wasm
    
    cd ../..
    
    echo -e "${GREEN}✓ Frontend built successfully${NC}"
}

# Build FFI bridge
build_ffi() {
    echo -e "\n${YELLOW}Building C/Rust FFI bridge...${NC}"
    
    cd shared
    
    # Build Rust library
    echo "Building Rust library..."
    cargo build --release
    
    # Generate C headers
    echo "Generating C headers..."
    cbindgen --config cbindgen.toml --crate shared --output include/defiant_ffi.h
    
    # Create static library
    echo "Creating static library..."
    ar rcs libdefiant_ffi.a target/release/libshared.a
    
    cd ..
    
    echo -e "${GREEN}✓ FFI bridge built successfully${NC}"
}

# Run database migrations
run_migrations() {
    echo -e "\n${YELLOW}Running database migrations...${NC}"
    
    # Check if Docker is available
    if command -v docker &> /dev/null; then
        echo "Using Docker for database..."
        if [ ! -f docker-compose.yml ]; then
            cd docker
        fi
        
        # Start PostgreSQL
        docker-compose up -d postgres
        
        # Wait for database to be ready
        echo "Waiting for database to be ready..."
        sleep 5
        
        # Run migrations
        cd ../backend
        DATABASE_URL="postgres://defiant:defiant123@localhost:5432/defiant" cargo sqlx migrate run
        
        cd ..
        
        echo -e "${GREEN}✓ Migrations completed${NC}"
    else
        echo -e "${YELLOW}Skipping migrations (Docker not available)${NC}"
    fi
}

# Generate SSL certificates
generate_certs() {
    echo -e "\n${YELLOW}Generating SSL certificates...${NC}"
    
    mkdir -p docker/nginx/ssl
    
    # Generate self-signed certificate for development
    openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
        -keyout docker/nginx/ssl/key.pem \
        -out docker/nginx/ssl/cert.pem \
        -subj "/C=US/ST=State/L=City/O=Defiant/CN=localhost"
    
    echo -e "${GREEN}✓ SSL certificates generated${NC}"
}

# Create distribution package
create_package() {
    echo -e "\n${YELLOW}Creating distribution package...${NC}"
    
    DIST_DIR="dist/defiant-$(date +%Y%m%d-%H%M%S)"
    mkdir -p "$DIST_DIR"
    
    # Copy backend binary
    cp backend/target/release/defiant-backend "$DIST_DIR/"
    
    # Copy frontend files
    mkdir -p "$DIST_DIR/web"
    cp -r frontend/web/* "$DIST_DIR/web/"
    cp frontend/build/defiant_wasm.js "$DIST_DIR/web/"
    cp frontend/build/defiant_wasm.wasm "$DIST_DIR/web/"
    
    # Copy configuration files
    cp -r backend/config "$DIST_DIR/"
    cp -r docker "$DIST_DIR/"
    cp docker-compose.yml "$DIST_DIR/" 2>/dev/null || true
    
    # Copy documentation
    cp README.md "$DIST_DIR/"
    cp LICENSE "$DIST_DIR/"
    
    # Create startup script
    cat > "$DIST_DIR/start.sh" << 'EOF'
#!/bin/bash
# Defiant Startup Script

echo "Starting Defiant Payments Platform..."

# Check for Docker
if command -v docker &> /dev/null; then
    echo "Starting with Docker Compose..."
    docker-compose up -d
    echo "Defiant is running at http://localhost"
else
    echo "Starting standalone..."
    ./defiant-backend &
    echo "Backend running at http://localhost:8080"
    echo "Frontend available in web directory"
fi

echo " Defiant started successfully!"
EOF
    chmod +x "$DIST_DIR/start.sh"
    
    # Create install script
    cat > "$DIST_DIR/install.sh" << 'EOF'
#!/bin/bash
# Defiant Installation Script

set -e

echo "Installing Defiant Payments Platform..."

# Check for dependencies
if ! command -v docker &> /dev/null; then
    echo "Error: Docker is required for installation"
    exit 1
fi

# Create data directories
mkdir -p data/postgres data/redis data/prometheus data/grafana

# Start services
docker-compose up -d

# Wait for services to be ready
echo "Waiting for services to start..."
sleep 10

# Run database migrations
docker-compose exec backend cargo sqlx migrate run

echo " Defiant installed successfully!"
echo ""
echo "Access URLs:"
echo "  - Frontend: http://localhost"
echo "  - Backend API: http://localhost/api"
echo "  - Metrics: http://localhost:9090"
echo "  - Grafana: http://localhost:3000 (admin/admin)"
echo ""
echo "To stop: docker-compose down"
EOF
    chmod +x "$DIST_DIR/install.sh"
    
    # Create tarball
    tar -czf "${DIST_DIR}.tar.gz" -C dist "$(basename "$DIST_DIR")"
    
    echo -e "${GREEN}✓ Distribution package created: ${DIST_DIR}.tar.gz${NC}"
}

# Main build process
main() {
    echo -e "${BLUE}Starting build process...${NC}"
    
    # Check prerequisites
    check_prerequisites
    
    # Build components
    build_backend
    build_frontend
    build_ffi
    
    # Setup
    run_migrations
    generate_certs
    
    # Create package
    create_package
    
    echo -e "\n${GREEN}======================================${NC}"
    echo -e "${GREEN} Defiant build completed successfully!${NC}"
    echo -e "${GREEN}======================================${NC}"
    echo ""
    echo "To start Defiant:"
    echo "  ./dist/*/start.sh"
    echo ""
    echo "Or with Docker Compose:"
    echo "  docker-compose up -d"
    echo ""
    echo "Access the application at:"
    echo "  Frontend: http://localhost"
    echo "  API: http://localhost:8080"
}

# Run main function
main "$@"
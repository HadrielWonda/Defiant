#!/bin/bash

# Defiant Deployment Script
# Deploys to various environments

set -e
set -o pipefail

# Configuration
ENVIRONMENT=${1:-"staging"}
VERSION=${2:-"latest"}
REGISTRY="your-registry.com/defiant"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Load environment configuration
load_config() {
    local env=$1
    CONFIG_FILE="deploy/config/${env}.env"
    
    if [ ! -f "$CONFIG_FILE" ]; then
        echo -e "${RED}Error: Configuration file not found: $CONFIG_FILE${NC}"
        exit 1
    fi
    
    source "$CONFIG_FILE"
    echo -e "${GREEN}✓ Loaded configuration for $env${NC}"
}

# Build and push Docker images
build_and_push() {
    echo -e "\n${YELLOW}Building Docker images...${NC}"
    
    # Build backend
    echo "Building backend image..."
    docker build -t "$REGISTRY/backend:$VERSION" -f docker/backend.Dockerfile ./backend
    
    # Build frontend
    echo "Building frontend image..."
    docker build -t "$REGISTRY/frontend:$VERSION" -f docker/frontend.Dockerfile ./frontend
    
    # Build nginx
    echo "Building nginx image..."
    docker build -t "$REGISTRY/nginx:$VERSION" -f docker/nginx.Dockerfile ./docker
    
    # Push images
    if [ "$ENVIRONMENT" != "local" ]; then
        echo "Pushing images to registry..."
        docker push "$REGISTRY/backend:$VERSION"
        docker push "$REGISTRY/frontend:$VERSION"
        docker push "$REGISTRY/nginx:$VERSION"
    fi

    echo -e "${GREEN} Docker images built and pushed${NC}"
}

# Deploy to Kubernetes
deploy_kubernetes() {
    echo -e "\n${YELLOW}Deploying to Kubernetes...${NC}"
    
    # Check for kubectl
    if ! command -v kubectl &> /dev/null; then
        echo -e "${RED}Error: kubectl not found${NC}"
        exit 1
    fi
    
    # Create namespace
    echo "Creating namespace..."
    kubectl create namespace defiant-$ENVIRONMENT --dry-run=client -o yaml | kubectl apply -f -
    
    # Create secrets
    echo "Creating secrets..."
    kubectl create secret generic defiant-secrets \
        --namespace=defiant-$ENVIRONMENT \
        --from-literal=database-url="$DATABASE_URL" \
        --from-literal=redis-url="$REDIS_URL" \
        --from-literal=jwt-secret="$JWT_SECRET" \
        --dry-run=client -o yaml | kubectl apply -f -
    
    # Create config map
    echo "Creating config map..."
    kubectl create configmap defiant-config \
        --namespace=defiant-$ENVIRONMENT \
        --from-file=deploy/config/$ENVIRONMENT.toml \
        --dry-run=client -o yaml | kubectl apply -f -
    
    # Apply deployments
    echo "Applying deployments..."
    
    # Process templates
    for file in deploy/kubernetes/*.yaml; do
        # Replace variables in template
        sed -e "s|{{REGISTRY}}|$REGISTRY|g" \
            -e "s|{{VERSION}}|$VERSION|g" \
            -e "s|{{ENVIRONMENT}}|$ENVIRONMENT|g" \
            "$file" | kubectl apply -f -
    done
    
    # Wait for rollout
    echo "Waiting for rollout to complete..."
    kubectl rollout status deployment/defiant-backend -n defiant-$ENVIRONMENT --timeout=300s
    kubectl rollout status deployment/defiant-frontend -n defiant-$ENVIRONMENT --timeout=300s
    
    echo -e "${GREEN} Kubernetes deployment complete${NC}"
}

# Deploy to Docker Swarm
deploy_swarm() {
    echo -e "\n${YELLOW}Deploying to Docker Swarm...${NC}"
    
    # Check for Docker Swarm
    if ! docker node ls &> /dev/null; then
        echo -e "${RED}Error: Not in Docker Swarm mode${NC}"
        exit 1
    fi
    
    # Create network
    echo "Creating network..."
    docker network create -d overlay defiant-network --attachable 2>/dev/null || true
    
    # Create secrets
    echo "Creating secrets..."
    echo "$DATABASE_URL" | docker secret create defiant_database_url - 2>/dev/null || true
    echo "$REDIS_URL" | docker secret create defiant_redis_url - 2>/dev/null || true
    echo "$JWT_SECRET" | docker secret create defiant_jwt_secret - 2>/dev/null || true
    
    # Deploy stack
    echo "Deploying stack..."
    docker stack deploy -c deploy/docker-swarm.yml defiant-$ENVIRONMENT
    
    # Wait for services
    echo "Waiting for services..."
    sleep 10
    
    # Check service status
    docker stack services defiant-$ENVIRONMENT
    
    echo -e "${GREEN} Docker Swarm deployment complete${NC}"
}

# Deploy to AWS
deploy_aws() {
    echo -e "\n${YELLOW}Deploying to AWS...${NC}"
    
    # Check for AWS CLI
    if ! command -v aws &> /dev/null; then
        echo -e "${RED}Error: AWS CLI not found${NC}"
        exit 1
    fi
    
    # Deploy ECS
    echo "Deploying to ECS..."
    aws ecs update-service \
        --cluster defiant-$ENVIRONMENT \
        --service defiant-service \
        --force-new-deployment \
        --region "$AWS_REGION"
    
    # Wait for deployment
    echo "Waiting for deployment..."
    aws ecs wait services-stable \
        --cluster defiant-$ENVIRONMENT \
        --services defiant-service \
        --region "$AWS_REGION"
    
    echo -e "${GREEN} AWS deployment complete${NC}"
}

# Run database migrations
run_migrations() {
    echo -e "\n${YELLOW}Running database migrations...${NC}"
    
    # Different methods based on environment
    case $ENVIRONMENT in
        "local")
            cd backend && DATABASE_URL="$DATABASE_URL" cargo sqlx migrate run
            ;;
        "kubernetes")
            kubectl run defiant-migrate \
                --namespace=defiant-$ENVIRONMENT \
                --image="$REGISTRY/backend:$VERSION" \
                --restart=Never \
                --command -- /app/migrate.sh
            ;;
        "swarm")
            docker service create \
                --name defiant-migrate \
                --network defiant-network \
                --secret defiant_database_url \
                --env "DATABASE_URL=$(cat /run/secrets/defiant_database_url)" \
                "$REGISTRY/backend:$VERSION" \
                /app/migrate.sh
            ;;
    esac

    echo -e "${GREEN} Migrations completed${NC}"
}

# Health check
health_check() {
    echo -e "\n${YELLOW}Running health check...${NC}"
    
    local url=""
    case $ENVIRONMENT in
        "local") url="http://localhost:8080/health" ;;
        "staging") url="https://staging.defiant.com/health" ;;
        "production") url="https://api.defiant.com/health" ;;
    esac
    
    if [ -n "$url" ]; then
        for i in {1..30}; do
            if curl -f -s "$url" > /dev/null; then
                echo -e "${GREEN} Health check passed${NC}"
                return 0
            fi
            echo "Attempt $i/30: Service not ready..."
            sleep 5
        done
        
        echo -e "${RED} Health check failed${NC}"
        return 1
    fi
    
    echo -e "${YELLOW} Skipping health check (no URL configured)${NC}"
    return 0
}

# Main deployment process
main() {
    echo -e "${BLUE} Deploying Defiant to $ENVIRONMENT${NC}"
    echo "======================================"
    
    # Load configuration
    load_config "$ENVIRONMENT"
    
    # Build and push images
    build_and_push
    
    # Run migrations
    run_migrations
    
    # Deploy based on environment
    case $DEPLOYMENT_TARGET in
        "kubernetes")
            deploy_kubernetes
            ;;
        "swarm")
            deploy_swarm
            ;;
        "aws")
            deploy_aws
            ;;
        "local")
            echo -e "${YELLOW}Local deployment - starting Docker Compose...${NC}"
            docker-compose -f docker/docker-compose.yml up -d
            ;;
        *)
            echo -e "${RED}Error: Unknown deployment target: $DEPLOYMENT_TARGET${NC}"
            exit 1
            ;;
    esac
    
    # Health check
    health_check
    
    echo -e "\n${GREEN}======================================${NC}"
    echo -e "${GREEN} Defiant deployed successfully to $ENVIRONMENT!${NC}"
    echo -e "${GREEN}======================================${NC}"
    echo ""
    
    # Print access information
    case $ENVIRONMENT in
        "staging")
            echo "Staging URLs:"
            echo "  - Frontend: https://staging.defiant.com"
            echo "  - API: https://api.staging.defiant.com"
            ;;
        "production")
            echo "Production URLs:"
            echo "  - Frontend: https://defiant.com"
            echo "  - API: https://api.defiant.com"
            echo "  - Status: https://status.defiant.com"
            ;;
    esac
}

# Run main function
main "$@"
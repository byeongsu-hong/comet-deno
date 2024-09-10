# Go parameters
GOCMD = go
GOBUILD = $(GOCMD) build
GOCLEAN = $(GOCMD) clean
GOTEST = $(GOCMD) test
GOGET = $(GOCMD) get

# Main target
all: build

# Build the project
build:
	$(GOBUILD) -o bin/kvstore main.go

# Clean the project
clean:
	$(GOCLEAN)
	rm -f bin/kvstore

# Run tests
test:
	$(GOTEST) -v ./...

# Install dependencies
deps:
	$(GOGET) github.com/example/dependency

# Run the project
run:
	$(GOBUILD) -o bin/kvstore main.go
	./bin/kvstore

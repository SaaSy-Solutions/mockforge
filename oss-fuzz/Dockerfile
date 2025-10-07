FROM gcr.io/oss-fuzz-base/base-builder-rust

# Install dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    git \
    && rm -rf /var/lib/apt/lists/*

# Clone the project repository
RUN git clone --depth 1 https://github.com/rclanan/mockforge $SRC/mockforge

# Set working directory
WORKDIR $SRC/mockforge

# Copy build script
COPY build.sh $SRC/

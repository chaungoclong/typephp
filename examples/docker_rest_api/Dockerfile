FROM alpine:latest

WORKDIR /app

# Copy only the installer script first (no Rust, PHP, or Composer pre-installed!)
COPY install/install.sh /tmp/install.sh

# Run the universal installer (auto-installs PHP 8.4, Composer, PHPStan L9, Rust, and tphpc)
RUN sh /tmp/install.sh && rm -f /tmp/install.sh

ENV PATH="/root/.typephp/bin:/root/.cargo/bin:${PATH}"

COPY examples/docker_rest_api/api.tphp /app/api.tphp

# Compile api.tphp -> api.php (with mandatory PHPStan Level 9 gate)
RUN tphpc /app/api.tphp --bundle --minify -o /app/api.php

EXPOSE 8080
CMD ["php", "-S", "0.0.0.0:8080", "/app/api.php"]

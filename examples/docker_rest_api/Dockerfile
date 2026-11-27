FROM alpine:latest

WORKDIR /app

# Copy only the installer script and our small .tphp REST API
# (No Rust, no PHP, no Composer pre-installed in alpine:latest!)
COPY install/install.sh /tmp/install.sh
COPY examples/docker_rest_api/api.tphp /app/api.tphp

# Run the universal installer (installs PHP 8.4, Composer, PHPStan L9, Rust, and tphpc)
RUN sh /tmp/install.sh && rm -f /tmp/install.sh

ENV PATH="/root/.typephp/bin:/root/.cargo/bin:${PATH}"

# Compile api.tphp -> api.php (with mandatory PHPStan Level 9 gate)
RUN tphpc /app/api.tphp --bundle --minify -o /app/api.php

EXPOSE 8080
CMD ["php", "-S", "0.0.0.0:8080", "/app/api.php"]

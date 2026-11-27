#!/bin/sh
set -eu

# TypePHP (tphpc) Universal Installer (POSIX sh: Linux/Alpine/macOS)
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/chaungoclong/typephp/main/install/install.sh | sh

REPO="${TPHPC_REPO:-chaungoclong/typephp}"
VERSION="${TPHPC_VERSION:-latest}"
INSTALL_DIR="${TPHPC_INSTALL_DIR:-$HOME/.typephp/bin}"
TYPEPHP_HOME="${TPHPC_HOME:-$HOME/.typephp}"
FROM_SOURCE=0
UNINSTALL=0

for arg in "$@"; do
  case "$arg" in
    --from-source) FROM_SOURCE=1 ;;
    --uninstall) UNINSTALL=1 ;;
    -y|--yes) ;;
    --help|-h)
      cat <<EOF
TypePHP (tphpc) Universal Installer

Usage:
  install.sh [OPTIONS]

Options:
  -y, --yes        Non-interactive mode
  --from-source    Force building tphpc from source (auto-installs Rust if missing)
  --uninstall      Remove tphpc and ~/.typephp
  -h, --help       Show this help message
EOF
      exit 0
      ;;
    *)
      echo "Unknown option: $arg" >&2
      exit 1
      ;;
  esac
done

log() { printf "[typephp] %s\n" "$*"; }

TMP_DIR=""
cleanup() {
  if [ -n "${TMP_DIR:-}" ] && [ -d "$TMP_DIR" ]; then
    rm -rf "$TMP_DIR"
  fi
}
trap cleanup EXIT

if [ "$UNINSTALL" -eq 1 ]; then
  rm -f "$INSTALL_DIR/tphpc" "$INSTALL_DIR/composer"
  rm -rf "$TYPEPHP_HOME"
  log "Uninstalled TypePHP from $TYPEPHP_HOME and $INSTALL_DIR"
  exit 0
fi

detect_platform() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os" in
    Linux)  os="linux" ;;
    Darwin) os="macos" ;;
    *) echo "Unsupported OS: $os" >&2; exit 1 ;;
  esac
  case "$arch" in
    x86_64|amd64)  arch="x86_64" ;;
    aarch64|arm64) arch="aarch64" ;;
    *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
  esac
  printf "%s-%s" "$os" "$arch"
}

run_pkg_install() {
  if [ "$(id -u)" -eq 0 ]; then
    "$@"
  elif command -v sudo >/dev/null 2>&1; then
    sudo "$@"
  else
    return 1
  fi
}

ensure_base_tools() {
  if command -v apk >/dev/null 2>&1; then
    if ! command -v curl >/dev/null 2>&1 || ! command -v gcc >/dev/null 2>&1 || ! command -v git >/dev/null 2>&1; then
      log "Installing base build tools on Alpine (curl, git, build-base)..."
      run_pkg_install apk add --no-cache curl ca-certificates git build-base
    fi
  elif command -v apt-get >/dev/null 2>&1; then
    if ! command -v curl >/dev/null 2>&1 || ! command -v cc >/dev/null 2>&1; then
      log "Installing base build tools (curl, git, build-essential)..."
      run_pkg_install apt-get update
      run_pkg_install apt-get install -y curl ca-certificates git build-essential
    fi
  fi
}

ensure_rust() {
  if [ -f "$HOME/.cargo/env" ]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
  fi
  if command -v cargo >/dev/null 2>&1; then
    return 0
  fi
  log "Rust/Cargo not found. Installing Rust automatically via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
  log "Rust installed: $(rustc --version)"
}

ensure_php() {
  if command -v php >/dev/null 2>&1; then
    return 0
  fi
  log "PHP not found. Attempting automatic installation..."
  if [ "$(uname -s)" = "Darwin" ] && command -v brew >/dev/null 2>&1; then
    brew install php
  elif command -v apk >/dev/null 2>&1; then
    run_pkg_install apk add --no-cache \
      php84 php84-cli php84-pdo php84-pdo_sqlite php84-sqlite3 \
      php84-phar php84-openssl php84-mbstring php84-tokenizer \
      php84-iconv php84-ctype php84-dom php84-xml php84-json || true
    if ! command -v php >/dev/null 2>&1 && [ -x /usr/bin/php84 ]; then
      run_pkg_install ln -sf /usr/bin/php84 /usr/bin/php
    fi
  elif command -v apt-get >/dev/null 2>&1; then
    run_pkg_install apt-get update
    run_pkg_install apt-get install -y php-cli php-sqlite3 php-pdo php-mbstring php-xml unzip curl
  elif command -v dnf >/dev/null 2>&1; then
    run_pkg_install dnf install -y php-cli php-pdo php-json unzip curl
  elif command -v pacman >/dev/null 2>&1; then
    run_pkg_install pacman -Sy --noconfirm php php-sqlite unzip curl
  else
    log "Warning: Could not auto-install PHP 8.1+. Please install PHP manually."
  fi
}

ensure_composer_and_phpstan() {
  mkdir -p "$INSTALL_DIR" "$TYPEPHP_HOME"
  if ! command -v php >/dev/null 2>&1; then
    return 0
  fi

  local composer_bin
  if command -v composer >/dev/null 2>&1; then
    composer_bin="$(command -v composer)"
  else
    log "Composer not found. Installing Composer into $INSTALL_DIR/composer..."
    TMP_DIR="$(mktemp -d)"
    curl -fsSL https://getcomposer.org/installer -o "$TMP_DIR/composer-setup.php"
    php "$TMP_DIR/composer-setup.php" --install-dir="$INSTALL_DIR" --filename=composer
    rm -rf "$TMP_DIR"
    TMP_DIR=""
    composer_bin="$INSTALL_DIR/composer"
  fi

  if [ ! -f "$TYPEPHP_HOME/vendor/bin/phpstan" ]; then
    log "Installing global PHPStan Level 9 tool into $TYPEPHP_HOME..."
    if [ ! -f "$TYPEPHP_HOME/composer.json" ]; then
      printf '{"name":"typephp/global-tools","require":{"phpstan/phpstan":"^2.0"}}\n' > "$TYPEPHP_HOME/composer.json"
    fi
    "$composer_bin" --working-dir="$TYPEPHP_HOME" install --no-interaction --prefer-dist --quiet || true
  fi
}

build_from_source() {
  ensure_rust
  local script_dir repo_root
  script_dir="$(cd "$(dirname "$0")" && pwd)"
  if [ -f "$script_dir/../Cargo.toml" ]; then
    repo_root="$(cd "$script_dir/.." && pwd)"
    log "Building tphpc from local source ($repo_root)..."
    cargo build --release --manifest-path "$repo_root/Cargo.toml"
    mkdir -p "$INSTALL_DIR"
    cp "$repo_root/target/release/tphpc" "$INSTALL_DIR/tphpc"
    chmod +x "$INSTALL_DIR/tphpc"
  else
    TMP_DIR="$(mktemp -d)"
    log "Cloning https://github.com/$REPO.git..."
    git clone --depth 1 "https://github.com/$REPO.git" "$TMP_DIR/typephp"
    cargo build --release --manifest-path "$TMP_DIR/typephp/Cargo.toml"
    mkdir -p "$INSTALL_DIR"
    cp "$TMP_DIR/typephp/target/release/tphpc" "$INSTALL_DIR/tphpc"
    chmod +x "$INSTALL_DIR/tphpc"
    rm -rf "$TMP_DIR"
    TMP_DIR=""
  fi
}

install_from_release() {
  local target="$1"
  local asset="tphpc-${target}.tar.gz"
  local url
  if [ "$VERSION" = "latest" ]; then
    url="https://github.com/${REPO}/releases/latest/download/${asset}"
  else
    url="https://github.com/${REPO}/releases/download/${VERSION}/${asset}"
  fi

  TMP_DIR="$(mktemp -d)"
  log "Downloading prebuilt binary $url..."
  if ! curl -fsSL "$url" -o "$TMP_DIR/$asset" 2>/dev/null; then
    rm -rf "$TMP_DIR"
    TMP_DIR=""
    return 1
  fi
  tar -xzf "$TMP_DIR/$asset" -C "$TMP_DIR"
  mkdir -p "$INSTALL_DIR"
  cp "$TMP_DIR/tphpc" "$INSTALL_DIR/tphpc"
  chmod +x "$INSTALL_DIR/tphpc"
  rm -rf "$TMP_DIR"
  TMP_DIR=""
  return 0
}

configure_shell_path() {
  local export_line="export PATH=\"$INSTALL_DIR:\$PATH\""
  for rc in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.profile"; do
    if [ -f "$rc" ]; then
      if ! grep -Fq "$INSTALL_DIR" "$rc"; then
        printf '\n# TypePHP\n%s\n' "$export_line" >> "$rc"
        log "Added $INSTALL_DIR to $rc"
      fi
    fi
  done
}

TARGET="$(detect_platform)"
log "Detected platform: $TARGET"
ensure_base_tools
ensure_php
ensure_composer_and_phpstan

if [ "$FROM_SOURCE" -eq 1 ]; then
  build_from_source
else
  if ! install_from_release "$TARGET"; then
    log "No prebuilt release binary found for $TARGET; auto-installing Rust & building from source..."
    build_from_source
  fi
fi

configure_shell_path
log "TypePHP installed successfully at $INSTALL_DIR/tphpc"
log "Run 'export PATH=\"$INSTALL_DIR:\$PATH\"' (or restart your terminal) and run 'tphpc --help'."

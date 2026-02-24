# Judge server for production (Hetzner/VPS)
# Builds the Rust binary and runs it with required toolchains.

FROM rust:1-slim-bookworm

WORKDIR /app
COPY . .

# Toolchains needed by the judges (adjust if you don't need some languages)
#
# Kotlin: we install the official compiler zip because Debian repos often don't
# ship a recent `kotlinc`.
ARG KOTLIN_VERSION=2.3.10

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl unzip \
    gcc libc6-dev \
    python3 \
    default-jdk-headless \
  && rm -rf /var/lib/apt/lists/*

# Install Kotlin compiler (kotlinc) to /opt/kotlin and symlink into PATH
RUN curl -fsSL -o /tmp/kotlin.zip \
      "https://github.com/JetBrains/kotlin/releases/download/v${KOTLIN_VERSION}/kotlin-compiler-${KOTLIN_VERSION}.zip" \
  && mkdir -p /opt/kotlin \
  && unzip -q /tmp/kotlin.zip -d /opt/kotlin \
  && ln -sf /opt/kotlin/kotlinc/bin/kotlinc /usr/local/bin/kotlinc \
  && rm -f /tmp/kotlin.zip

# Build only the judge server binary
RUN cargo build --release --bin summer_quiz_judge_server

ENV JUDGE_BIND=0.0.0.0:8787
EXPOSE 8787

CMD ["./target/release/summer_quiz_judge_server"]


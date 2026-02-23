# ============================================================
# summer_quiz judge server – multi-stage Docker build
# ============================================================
# Build:  docker build -t summer-quiz-judge .
# Run:    docker run -p 8787:8787 summer-quiz-judge
# ============================================================

# --- Stage 1: compile the server binary ----------------------
FROM rust:1.85-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin summer_quiz_judge_server

# --- Stage 2: minimal runtime with compilers ----------------
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    gcc libc6-dev \
    python3 \
    default-jdk-headless \
    ca-certificates \
  && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/summer_quiz_judge_server /usr/local/bin/

ENV JUDGE_BIND=0.0.0.0:8787
EXPOSE 8787

CMD ["summer_quiz_judge_server"]

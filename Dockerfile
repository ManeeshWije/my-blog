FROM lukemathwalker/cargo-chef:latest as chef

ARG DATABASE_URL

ENV DATABASE_URL=$DATABASE_URL

WORKDIR /app

FROM chef AS planner
COPY ./Cargo.toml ./Cargo.lock ./
COPY ./src ./src
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ARG DATABASE_URL
ENV DATABASE_URL=$DATABASE_URL
COPY --from=planner /app/recipe.json .
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release
RUN mv ./target/release/blog-rust ./app

FROM debian:latest AS runtime
WORKDIR /app
# Install necessary dependencies
RUN apt-get update && apt-get install -y libssl-dev ca-certificates
COPY --from=builder /app/app /usr/local/bin/
COPY ./public /app/public
COPY ./src /app/src
ENTRYPOINT ["/usr/local/bin/app"]

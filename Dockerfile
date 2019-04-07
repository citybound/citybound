FROM rust:1.32

RUN apt-get update
RUN apt-get install --yes \
      build-essential \
      git \
      curl \
      libssl-dev \
      pkg-config

# Workaround because npm run ensure-tooling fails to install cargo-web.
RUN cargo install cargo-web --version 0.6.16

COPY . /app
WORKDIR /app

RUN curl -sL https://deb.nodesource.com/setup_10.x | bash - && \
    apt-get install --yes \
      nodejs
RUN npm run ensure-tooling

RUN cd cb_browser_ui && \
    npm install && \
    npm run build

RUN npm run build-server

EXPOSE 1234
EXPOSE 9999

ENTRYPOINT set -xe && \
  target/release/citybound \
    --bind 0.0.0.0:1234 \
    --bind-sim 0.0.0.0:9999

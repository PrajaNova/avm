FROM rust:1-bookworm AS build

WORKDIR /workspace

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git curl \
    && rm -rf /var/lib/apt/lists/*

COPY . /workspace

RUN cargo test --workspace \
    && cargo build --package avm-cli --bin avm-bin

FROM debian:12-slim AS runtime

WORKDIR /workspace
RUN useradd --create-home --shell /bin/bash avm

ENV PATH="/usr/local/bin:${PATH}"

COPY --from=build /workspace/target/debug/avm-bin /usr/local/bin/avm-bin
COPY --from=build /workspace/bin/avm-bin.js /workspace/bin/avm-bin.js

RUN chmod +x /usr/local/bin/avm-bin

USER avm

CMD ["bash"]

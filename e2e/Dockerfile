# End-to-end tests on a clean Ubuntu. Two targets:
#   local (default): avm-bin built from this checkout, installed by this
#                    checkout's install.sh, so a PR tests its own code.
#   release:         nothing prebuilt; the tests install the published avm.
# amd64 because Google ships Android's Linux tools for x86_64 only (Docker
# on Apple Silicon runs this through Rosetta).

FROM --platform=linux/amd64 rust:1 AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release -p avm-cli --bin avm-bin \
 && mkdir /dist \
 && tar -C target/release -czf /dist/avm_linux_amd64.tar.gz avm-bin \
 && cd /dist && sha256sum avm_* > checksums.txt

FROM --platform=linux/amd64 ubuntu:24.04 AS release
RUN apt-get update \
 && apt-get install -y --no-install-recommends curl ca-certificates tar gzip unzip git \
 && rm -rf /var/lib/apt/lists/*
# A normal user, like a real install (no root, no sudo).
RUN useradd -m tester
COPY --chown=tester e2e/tests/ /e2e/
USER tester
WORKDIR /home/tester
ENV AVM_E2E_INSTALL=release
ENTRYPOINT ["bash", "/e2e/run-all.sh"]

FROM release AS local
COPY --from=build /dist /opt/dist
COPY install.sh /opt/install.sh
ENV AVM_E2E_INSTALL=local

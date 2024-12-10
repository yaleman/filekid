FROM debian:12 AS cargo_builder

# fixing the issue with getting OOMKilled in BuildKit
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

# install the dependencies
RUN apt-get update && apt-get upgrade -y && apt-get install -y \
    sccache \
    curl \
    git \
    clang \
    build-essential \
    procps \
    mold

# install rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
RUN mv /root/.cargo/bin/* /usr/local/bin/

RUN mkdir /filekid
COPY . /filekid/

WORKDIR /filekid

RUN ./scripts/copy_linker_config.sh

# ENV RUSTC_WRAPPER="/usr/bin/sccache"
ENV CC="/usr/bin/clang"
# # do the build bits
RUN cargo build --release --bins
RUN chmod +x /filekid/target/release/filekid

# https://github.com/GoogleContainerTools/distroless/blob/main/examples/rust/Dockerfile
FROM debian:12-slim AS filekid

COPY --from=cargo_builder /filekid/target/release/filekid /filekid
COPY ./static /static/
RUN useradd filekid
RUN chown -R filekid /static
RUN chgrp -R filekid /static
USER filekid
ENTRYPOINT ["/filekid"]

FROM cnbcool/default-dev-env:latest

ENV RUSTUP_DIST_SERVER="https://rsproxy.cn"
ENV RUSTUP_UPDATE_ROOT="https://rsproxy.cn/rustup"
ENV PATH="$HOME/.cargo/bin:$PATH"

RUN apt-get update && \
    apt-get install -y m4 file npm rustup && \
    apt-get install -y libgmp-dev libmpfr-dev libmpc-dev libflint-dev && \
    rm -rf /var/lib/apt/lists/*

RUN rustup install stable && rustup default stable

RUN mkdir -p $HOME/.cargo && \
    printf '[source.crates-io]\nreplace-with = "rsproxy-sparse"\n\n[source.rsproxy]\nregistry = "https://rsproxy.cn/crates.io-index"\n\n[source.rsproxy-sparse]\nregistry = "sparse+https://rsproxy.cn/index/"\n\n[registries.rsproxy]\nindex = "https://rsproxy.cn/crates.io-index"\n\n[net]\ngit-fetch-with-cli = true\n' > $HOME/.cargo/config.toml

RUN npm install -g @anthropic-ai/claude-code
RUN npm install -g --ignore-scripts @earendil-works/pi-coding-agent

RUN echo "=== Rust versions ===" && \
    rustc --version && \
    cargo --version && \
    echo "=== Node.js versions ===" && \
    node --version && \
    npm --version && \
    echo "=== Claude Code ===" && \
    claude --version || echo "claude-code installed" && \
    echo "=== Pi Agent ===" && \
    pi --version || echo "pi-coding-agent installed"

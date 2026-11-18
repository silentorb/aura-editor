# Experimental Aura Editor Tauri/dev image (repo-owned).
# Built from the workbench engine image via Compose additional_contexts.
# Not the Iced path. Future: CI home; may later FROM an Imp Lab LCD base.
ARG BASE_IMAGE=aura-workbench-engine:local
FROM ${BASE_IMAGE}

USER root

# Tauri Linux prerequisites (WebKitGTK runtime + build deps). No Iced/wgpu set.
RUN apt-get update -qq \
 && apt-get install -y -qq \
      build-essential \
      curl \
      wget \
      file \
      libwebkit2gtk-4.1-dev \
      libgtk-3-dev \
      libayatana-appindicator3-dev \
      librsvg2-dev \
      libssl-dev \
      libxdo-dev \
 && rm -rf /var/lib/apt/lists/*

USER vscode

# Bun (JS runtime + package manager) — no separate Node.js.
ARG BUN_VERSION=1.2.19
ENV BUN_INSTALL=/home/vscode/.bun
ENV PATH="${BUN_INSTALL}/bin:${PATH}"
RUN curl -fsSL "https://bun.sh/install" | bash -s "bun-v${BUN_VERSION}" \
 && bun --version

# Ensure bun is on PATH even when login shells reset ENV (compose exec -lc).
USER root
RUN ln -sf /home/vscode/.bun/bin/bun /usr/local/bin/bun \
 && ln -sf /home/vscode/.bun/bin/bunx /usr/local/bin/bunx
USER vscode

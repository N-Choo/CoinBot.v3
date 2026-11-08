# opencode serve sandbox: its own network namespace, webfetch allowed,
# host-localhost unreachable from inside. Secrets are runtime-only (-e / .env).
FROM node:24-slim

RUN apt-get update \
	&& apt-get install -y --no-install-recommends git curl ripgrep ca-certificates \
	&& rm -rf /var/lib/apt/lists/*

RUN npm i -g opencode-ai@1.19.0

WORKDIR /app

CMD ["opencode", "serve", "--hostname", "0.0.0.0", "--port", "4096"]

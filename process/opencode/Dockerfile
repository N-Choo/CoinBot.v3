# opencode serve sandbox: its own network namespace, webfetch allowed,
# host-localhost unreachable from inside. Secrets are runtime-only (-e / .env).
FROM node:24-slim

RUN apt-get update \
	&& apt-get install -y --no-install-recommends git curl ripgrep ca-certificates \
	&& rm -rf /var/lib/apt/lists/*

RUN npm i -g opencode-ai@1.19.0

WORKDIR /app

COPY entrypoint.sh /usr/local/bin/opencode-entrypoint.sh
RUN chmod +x /usr/local/bin/opencode-entrypoint.sh

ENTRYPOINT ["/usr/local/bin/opencode-entrypoint.sh"]

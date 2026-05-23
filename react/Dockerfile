# ── Build stage ──
FROM node:22-alpine AS builder
WORKDIR /app
COPY package*.json ./
RUN npm i -g npm@latest && npm install
COPY . .
RUN npm run build

# ── Production stage ──
FROM nginx:1.27-alpine
COPY nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /app/dist /usr/share/nginx/html
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]

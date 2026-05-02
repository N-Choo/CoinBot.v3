# Use Node 22 (LTS) - includes modern npm by default
FROM node:22-alpine

WORKDIR /app

# We skip 'npm install -g npm' because Node 22 is already up to date
COPY package*.json ./
RUN npm install

EXPOSE 5173
CMD ["npm", "run", "dev", "--", "--host", "0.0.0.0"]

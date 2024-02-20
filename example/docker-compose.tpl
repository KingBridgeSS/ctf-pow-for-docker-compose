services:
  nginx-service:
    image: nginx:alpine
    ports:
      - {{port}}:80
FROM nginx:alpine
COPY nginx.conf /etc/nginx/conf.d/default.conf
COPY index.html /usr/share/nginx/html/
COPY install.sh /usr/share/nginx/html/
COPY install.ps1 /usr/share/nginx/html/
COPY install-skills.sh /usr/share/nginx/html/
COPY install-skills.ps1 /usr/share/nginx/html/
EXPOSE 8080
CMD ["nginx", "-g", "daemon off;"]

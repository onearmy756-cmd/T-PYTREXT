#!/bin/bash
# ═══════════════════════════════════════════════════════════════
#  T-PYTREXT SERVER DEPLOYMENT SCRIPT
#  Auto-deploy kwenye VPS yoyote (Ubuntu/Debian)
# ═══════════════════════════════════════════════════════════════
set -e

APP_NAME="${1:-t-pytrext}"
DOMAIN="${2:-}"
PORT="${3:-8080}"

echo "═══════════════════════════════════════════════"
echo "  🚀 T-PYTREXT SERVER DEPLOYER"
echo "  App: $APP_NAME | Port: $PORT"
echo "═══════════════════════════════════════════════"

# ═══ STEP 1: Update System ═══
echo ""
echo "━━━ 1/6: Updating System ━━━"
apt-get update -y && apt-get upgrade -y
apt-get install -y curl wget git nginx certbot python3-certbot-nginx

# ═══ STEP 2: Install Python ═══
echo ""
echo "━━━ 2/6: Installing Python ━━━"
apt-get install -y python3 python3-pip python3-venv

# ═══ STEP 3: Setup App ═══
echo ""
echo "━━━ 3/6: Setting Up App ━━━"
mkdir -p /opt/$APP_NAME
cd /opt/$APP_NAME

# Install PyTreXT (from PyPI or local)
pip install t-pytrext 2>/dev/null || pip install -e /root/T-PYTREXT 2>/dev/null || echo "⚠️ Install manually"

# Create run script
cat > run.sh << 'RUNEOF'
#!/bin/bash
cd /opt/$APP_NAME
export PYTREX_ENV=production
export PYTREX_PORT=${PORT:-8080}
python3 main.py
RUNEOF
chmod +x run.sh

# ═══ STEP 4: Setup systemd ═══
echo ""
echo "━━━ 4/6: Setup systemd Service ━━━"
cat > /etc/systemd/system/$APP_NAME.service << SERVICEEOF
[Unit]
Description=PyTreXT $APP_NAME Service
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/opt/$APP_NAME
ExecStart=/opt/$APP_NAME/run.sh
Restart=always
RestartSec=5
Environment=PYTREX_ENV=production
Environment=PYTREX_PORT=$PORT

[Install]
WantedBy=multi-user.target
SERVICEEOF

systemctl daemon-reload
systemctl enable $APP_NAME
systemctl start $APP_NAME

# ═══ STEP 5: Setup Nginx ═══
echo ""
echo "━━━ 5/6: Setup Nginx ━━━"
cat > /etc/nginx/sites-available/$APP_NAME << NGINXEOF
server {
    listen 80;
    server_name ${DOMAIN:-_};

    location / {
        proxy_pass http://127.0.0.1:$PORT;
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
    }
}
NGINXEOF

ln -sf /etc/nginx/sites-available/$APP_NAME /etc/nginx/sites-enabled/
nginx -t && systemctl reload nginx

# ═══ STEP 6: Firewall ═══
echo ""
echo "━━━ 6/6: Setup Firewall ━━━"
ufw allow 22/tcp
ufw allow 80/tcp
ufw allow 443/tcp
ufw allow $PORT/tcp
ufw --force enable

# ═══ DONE ═══
echo ""
echo "═══════════════════════════════════════════════"
echo "  ✅ DEPLOYMENT COMPLETE!"
echo ""
echo "  App: systemctl status $APP_NAME"
echo "  Logs: journalctl -u $APP_NAME -f"
echo "  URL: http://YOUR_SERVER_IP"
echo "═══════════════════════════════════════════════"

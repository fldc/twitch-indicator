#!/bin/bash

echo "=== Twitch Indicator Configuration Setup ==="
echo ""
echo "Du behöver skapa en Twitch application först:"
echo "1. Gå till: https://dev.twitch.tv/console"
echo "2. Logga in med ditt Twitch-konto"
echo "3. Klicka 'Register Your Application'"
echo "4. Fyll i:"
echo "   - Name: Twitch Indicator"
echo "   - OAuth Redirect URLs: https://localhost:17563"
echo "   - Category: Application Integration"
echo "5. Klicka 'Create'"
echo "6. Kopiera Client ID från den skapade applikationen"
echo ""

read -p "Ange din Twitch Client ID: " CLIENT_ID

if [ -z "$CLIENT_ID" ]; then
    echo "Error: Client ID kan inte vara tom"
    exit 1
fi

# Update the config file
mkdir -p ~/.config/twitch-indicator
cat > ~/.config/twitch-indicator/config.toml << EOF
[general]
autostart = false
minimize_to_tray = true

[twitch]
client_id = "$CLIENT_ID"
redirect_uri = "https://localhost:17563"
access_token = ""
refresh_token = ""
refresh_interval_minutes = 5

[notifications]
enabled = true
show_game = true
show_viewer_count = true
timeout_ms = 3000

[ui]
show_selected_channels_on_top = true
dark_theme = false

[stream_open]
arguments = []
extra_command = ""
extra_arguments = []
EOF

echo ""
echo "Konfiguration uppdaterad!"
echo "Du kan nu köra: ./target/debug/twitch-indicator"

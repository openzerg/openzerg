# OpenZerg Update Procedure

## 1. Modify Code

```bash
cd ~/openzerg
# Make changes to source files
```

## 2. Local Test

```bash
cd ~/openzerg
nix develop --command cargo build
nix develop --command cargo test
```

## 3. Nix Build

```bash
cd ~/openzerg
nix build
```

If build fails, fix issues and repeat step 2-3.

## 4. Commit Changes

```bash
cd ~/openzerg
git add .
git commit -m "descriptive message"
git push
```

## 5. Update Incus Container

```bash
# Update flake and rebuild
incus exec openzerg -- bash -c 'cd /etc/nixos && nix --extra-experimental-features "nix-command flakes" flake update && nixos-rebuild switch --flake .#default --option sandbox false'
```

## 6. CLI Test

```bash
# Test via API
SESSION_ID="test-$(date +%s)"
curl -s -X POST "http://192.168.200.42:8081/api/sessions/$SESSION_ID/chat" \
  -H "Content-Type: application/json" \
  -d '{"content":"What is 2+2?"}'
sleep 10
curl -s "http://192.168.200.42:8081/api/sessions"
```

## 7. Playwright Test

```bash
cd ~/playwright-test
rm -f openzerg-*.png
nix develop --command node test-openzerg-chat.js
```

Check screenshots in `~/playwright-test/openzerg-*.png`

## Notes

- Always use `nix develop --command` instead of `nix-shell -p`
- Container IP: `192.168.200.42`
- Port: `8081`
- Supported models: `glm-5`, `kimi-k2.5`
- SQLite database: `/var/lib/openzerg/.openzerg/openzerg.db`
# OpenRouteService (ORS) — Quick Setup Guide for UK Routing

## Prerequisites
- Linux system with Docker & Docker Compose installed
- ~8GB+ RAM (16GB+ recommended)
- ~50GB+ free disk space

---

## 1. Install Docker

```bash
sudo apt update && sudo apt install -y docker.io docker-compose-plugin
sudo systemctl enable --now docker
sudo usermod -aG docker $USER
newgrp docker          # Apply group change to current session
docker ps              # Verify (should show no error)
```

---

## 2. Create Project Directory

```bash
mkdir ~/ors-uk && cd ~/ors-uk
```

---

## 3. Download Docker Compose File

```bash
wget https://github.com/GIScience/openrouteservice/releases/latest/download/docker-compose.yml
```

---

## 4. Create Required Directories

```bash
mkdir -p ors-docker/config ors-docker/elevation_cache ors-docker/files ors-docker/graphs ors-docker/logs
```

---

## 5. Download UK OSM Data

```bash
wget -O ors-docker/files/great-britain-latest.osm.pbf   https://download.geofabrik.de/europe/great-britain-latest.osm.pbf
```

> ~2GB download. For smaller areas, use https://download.geofabrik.de/europe/great-britain/england/ instead.

---

## 6. Download & Configure ORS Config

```bash
wget -O ors-docker/config/ors-config.yml   https://raw.githubusercontent.com/GIScience/openrouteservice/main/ors-config.yml
```

Edit the config to point to your UK data:

```bash
nano ors-docker/config/ors-config.yml
```

Find and change:
```yaml
source_file: /home/ors/files/great-britain-latest.osm.pbf
```

> Optional: Enable additional profiles (foot-walking, cycling-regular, etc.) under `ors.engine.profiles`.

---

## 7. Fix File Permissions (Avoid Root-Owned Files)

Edit `docker-compose.yml`:

```bash
nano docker-compose.yml
```

Uncomment:
```yaml
user: "1000:1000"
```

---

## 8. Start OpenRouteService

```bash
docker compose up -d
```

---

## 9. Monitor Startup

Graph building takes **15–60 minutes** for UK data:

```bash
docker compose logs -tf
```

Wait for:
```
{"status":"ready"}
```

---

## 10. Verify Health

```bash
curl http://localhost:8080/ors/v2/health
```

Expected: `{"status":"ready"}`

---

## 11. Test a UK Route

```bash
curl -X POST 'http://localhost:8080/ors/v2/directions/driving-car'   -H 'Content-Type: application/json'   -d '{"coordinates":[[-0.1276,51.5074],[-0.0877,51.5133]]}'
```

(London → Tower Bridge)

---

## Useful Commands

| Command | Description |
|---------|-------------|
| `docker compose up -d` | Start ORS |
| `docker compose down` | Stop & remove container |
| `docker compose stop` | Stop only |
| `docker compose logs -tf` | Follow logs live |
| `docker compose down && docker compose up -d` | Restart with config changes |

---

## Troubleshooting

| Issue | Fix |
|-------|-----|
| Permission denied on Docker socket | Run `newgrp docker` or log out & back in |
| Out of memory | Increase RAM or reduce enabled profiles |
| Graph rebuild needed | Set `REBUILD_GRAPHS=True` in `docker-compose.yml` or delete `ors-docker/graphs/` |
| Config not loading | Check `ors-docker/config/ors-config.yml` exists and path is correct |

---

## File Locations (Inside Container)

| Host Path | Container Path | Purpose |
|-----------|----------------|---------|
| `ors-docker/config/` | `/home/ors/config/` | Config files |
| `ors-docker/files/` | `/home/ors/files/` | OSM data |
| `ors-docker/graphs/` | `/home/ors/graphs/` | Built routing graphs |
| `ors-docker/logs/` | `/home/ors/logs/` | Application logs |
| `ors-docker/elevation_cache/` | `/home/ors/elevation_cache/` | Elevation data |

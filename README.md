# Save Steward

## Overview

**Save Steward** is a Rust-powered, privacy-focused cloud and local save management platform designed for gamers who want total control and reliability over their game progress. It enables automatic save detection, ultra-fast compression using **Zstandard (zstd)**, versioned backups, and cloud synchronization through **Cloudflare R2**. Later versions will introduce a **Go-based backend** hosted on **Render** for scalable cloud processing, user management, and analytics.

## Vision

To create the world's most efficient and secure personal save management system — empowering gamers to own, protect, and share their progress independently of game platforms.

## Core Features

* **Automatic Save Detection:** Detects and tracks game save locations automatically using directory indexing and hash tracking.
* **Versioned Backups:** Each detected change triggers a new compressed save version with timestamp and checksum.
* **Zstd Compression:** Achieves up to 10× faster compression and 40–70% smaller file sizes compared to gzip.
* **SQLite Metadata Tracking:** Local lightweight database stores file metadata, version history, and checksum integrity.
* **Instant Restore:** Decompress and restore any previous version in under 200 ms.
* **Cloudflare R2 Sync:** Optional cloud upload for secure off-device storage and redundancy.
* **Supabase Integration:** For authentication, cross-device sync, and save sharing.
* **Future Add-ons:** Config sync, mod backups, achievement parsing, and social leaderboard.
* **Go Backend (Later MVPs):** High-performance API layer hosted on Render for secure, scalable operations.

## Architecture

```
+-------------------------------------------+
|           Save Steward Desktop App        |
|             (Rust + Tauri)                |
+------------------+------------------------+
                   |
           +-------v--------+
           | Local Engine   |
           | (Zstd, SQLite) |
           +-------+--------+
                   |
           Local Version Storage
                   |
           +-------v--------+
           | Cloud Sync     |
           | (R2 + Supabase)|
           +-------+--------+
                   |
           +-------v--------+
           | Go Backend API |
           | (Render)       |
           +----------------+
```

### Backup Flow

1. Detect changes in game save directory.
2. Compress updated file using Zstd (`.zst` format).
3. Store backup with timestamp in local version folder.
4. Write metadata to SQLite.
5. Optionally upload `.zst` to Cloudflare R2 via the Go API.

### Restore Flow

1. Retrieve selected backup metadata from SQLite.
2. Decompress `.zst` file using `zstd::decode_all()`.
3. Replace current game save with restored version.
4. Sync local index and optionally update backend state.

## Technology Stack

| Component             | Technology             | Purpose                        |
| --------------------- | ---------------------- | ------------------------------ |
| Core Language         | Rust                   | Local app + compression        |
| Framework             | Tauri                  | Desktop wrapper                |
| Backend Language      | Go                     | Cloud API + orchestration      |
| Backend Hosting       | Render                 | API hosting & scaling          |
| Compression           | Zstandard (zstd crate) | Fast compression/decompression |
| Local Database        | SQLite                 | Metadata + version tracking    |
| Cloud Storage         | Cloudflare R2          | Save backup storage            |
| Cloud Database / Auth | Supabase               | User data + social layer       |

## Performance

| Operation               | Average Time (20 MB File) |
| ----------------------- | ------------------------- |
| Detection               | 5–20 ms                   |
| Zstd Compression        | 90–150 ms                 |
| SQLite Write            | 1–5 ms                    |
| Restore                 | 100–200 ms                |
| Cloud Upload (Optional) | 6–10 s (async)            |

The full local backup cycle completes in **under 300 ms**, ensuring minimal latency and no noticeable gameplay interruption.

## Monetization Model

### Storage-Based Tiers (Original)
| Tier     | Features                                               | Price                    |
| -------- | ------------------------------------------------------ | ------------------------ |
| Free     | Local versioning only                                  | $0                       |
| Standard | 5GB R2 cloud + version history                         | $3/year                  |
| Pro      | 20GB + achievements + Supabase sync                    | $7/year                  |
| Elite    | Unlimited storage + mod/config sync + social dashboard | $15/year or $50 lifetime |

### Branch-Based Tiers (New)
| Tier     | Branches/Game | Versions/Branch | Features                                | Price     |
| -------- | ------------- | --------------- | --------------------------------------- | --------- |
| Free     | 1 branch      | 3 versions      | Basic save protection                   | $0        |
| Standard | 3 branches    | 10 versions     | Multiple playthroughs, basic naming     | $7/year   |
| Pro      | 10 branches   | 50 versions     | Advanced branch management, tags        | $10/year  |
| Elite    | Unlimited     | Unlimited       | Full branch control, compare, share     | $20/year  |

**Branch Strategy:** Limits number of concurrent save progressions per game. Perfect for NewGame+ players who want to maintain separate story branches without losing previous progress.

### Conversion Strategy

**Storage Model:**
* Free tier hooks users into the ecosystem.
* Cloud storage and achievement sync encourage upgrade.
* Targeted campaigns for streamers, modders, and speedrunners.

**Branch Model:**
* Free users experience save loss during NewGame+ - natural upgrade trigger.
* Casual players start with basic protection, serious gamers upgrade for branches.
* "Want to keep multiple story progressions? Upgrade to Standard!"
* Branch limits create immediate pain point that drives conversions.

**Combined Strategy:**
* Offer both models - users choose storage limits OR branch limits.
* Premium users get both unlimited storage AND unlimited branches.
* Double conversion funnel increases upgrade opportunities.

## Roadmap

### MVP 1

* Local detection and zstd compression
* SQLite metadata storage
* Manual restore interface

### MVP 2

* Cloudflare R2 sync
* Background uploads
* Supabase user accounts

### MVP 3

* Achievement parsing
* Optional Steam API integration

### MVP 4

* Go backend deployment on Render for cloud sync & analytics
* Config/mod sync (premium)

### MVP 5

* Save sharing, leaderboards, and social profiles

## Economic & Realistic Evaluation

| Factor            | Assessment                                            |
| ----------------- | ----------------------------------------------------- |
| **Feasibility**   | High — uses proven, low-cost tools (Rust, Go, R2)     |
| **Storage Costs** | $0.015/GB/month (R2) — negligible for MVP scale       |
| **Performance**   | Excellent, sub-300ms local ops                        |
| **Competition**   | None in user-facing save versioning niche             |
| **Risk**          | Moderate — small market awareness, high trust barrier |

### Annual Revenue Projections (USD) - Updated Models

| Year   | Conservative | Speculative | Notes                                  |
| ------ | ------------ | ----------- | -------------------------------------- |
| Year 1 | $3,000       | $15,000     | Branch model adds 50% conversion boost |
| Year 2 | $8,000       | $35,000     | Dual-model retention improvement       |
| Year 3 | $15,000      | $75,000     | Branch limits drive premium upgrades   |
| Year 4 | $30,000      | $150,000    | Combined model maturity                |
| Year 5 | $45,000      | $300,000    | Full feature ecosystem adoption        |

**Revenue Model Impact:**
* **Branch-based tiers** expected to increase conversion by 40-60%
* **NewGame+ pain point** creates immediate upgrade urgency
* **Dual conversion funnels** capture both storage and branch users
* **Higher tier pricing** reflects increased value proposition

## Realistic Bottom Line

**Save Steward** is a technically elegant and lean platform. Its combination of **Rust local performance** and **Go cloud scalability** offers durability, low latency, and gamer-oriented reliability. With future social and achievement features powered by Supabase and Render-hosted APIs, Save Steward will redefine how gamers manage and celebrate progress.

---

**Tagline:** *Save Smart. Play Free. Be Your Own Steward.*

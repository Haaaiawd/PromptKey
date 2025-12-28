# Git Forensics Report
**Analysis Period**: 180 days | **Commits**: 28 | **Shallow**: False

## Global Coupling Analysis

### Top Coupled Pairs
| File A | File B | Frequency | Count | Risk |
|--------|--------|-----------|-------|------|
| `src/index.html` | `src/styles.css` | 0.89 | 8 | HIGH_COUPLING |
| `service/src/main.rs` | `src/main.rs` | 0.54 | 7 | MEDIUM_COUPLING |
| `src/main.rs` | `src/styles.css` | 0.54 | 7 | MEDIUM_COUPLING |
| `src/main.rs` | `src/main_simple.js` | 0.46 | 6 |  |
| `src/main_simple.js` | `src/styles.css` | 0.67 | 6 | MEDIUM_COUPLING |
| `src/index.html` | `src/main.rs` | 0.46 | 6 |  |
| `Cargo.toml` | `src/main.rs` | 0.38 | 5 |  |
| `service/src/main.rs` | `src/styles.css` | 0.56 | 5 | MEDIUM_COUPLING |
| `src/index.html` | `src/main_simple.js` | 0.56 | 5 | MEDIUM_COUPLING |
| `service/src/config/mod.rs` | `service/src/main.rs` | 0.57 | 4 |  |
| `service/src/config/mod.rs` | `src/main.rs` | 0.31 | 4 |  |
| `service/src/injector/mod.rs` | `src/main.rs` | 0.31 | 4 |  |
| `service/src/config/mod.rs` | `service/src/injector/mod.rs` | 0.44 | 4 |  |
| `service/src/db.rs` | `service/src/main.rs` | 0.57 | 4 | MEDIUM_COUPLING |
| `service/src/db.rs` | `src/main.rs` | 0.31 | 4 |  |
| `Cargo.toml` | `src/styles.css` | 0.44 | 4 |  |
| `service/src/db.rs` | `src/styles.css` | 0.44 | 4 |  |
| `src/main.rs` | `tauri.conf.json` | 0.31 | 4 |  |
| `service/src/main.rs` | `src/index.html` | 0.44 | 4 |  |
| `src/index.html` | `src/main.js` | 0.44 | 4 |  |

## Recommendations
- 发现 1 对高耦合生产文件，考虑合并或重构。
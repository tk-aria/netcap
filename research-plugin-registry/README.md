# research-plugin-registry

Custom plugin marketplace for Claude Code - multi-source web research, venue discovery, and cross-site fact-checking tools.

## Marketplace Installation

### Claude Code

**From GitHub (Remote):**
```bash
/plugin marketplace add granizm/research-plugin-registry
```

**From Local Path:**
```bash
/plugin marketplace add path/to/research-plugin-registry
```

---

## Available Plugins

| Plugin | Version | Description |
|--------|---------|-------------|
| research-tools | 1.4.0 | Multi-source web research - restaurant discovery, Wantedly/X(Twitter) developer recruitment, high-salary job search, 12-dimension company research, cross-site fact-checking |

---

## Plugin Management

### List Available Plugins

```bash
/plugin list
```

### Enable Plugin

```bash
/plugin enable research-tools@research-plugin-registry
```

### Disable Plugin (Keep Installed)

```bash
/plugin disable research-tools@research-plugin-registry
```

### Uninstall Plugin

```bash
/plugin uninstall research-tools@research-plugin-registry
```

### Reload Plugins (Apply Changes)

```bash
/reload-plugins
```

---

## Plugin Details

### research-tools

Multi-source web research plugin with modular skill architecture:

| Skill | Trigger | Description |
|-------|---------|-------------|
| `restaurant-finder` | "居酒屋探して", "レストラン検索", "宴会場所" | 複数グルメサイト横断検索・ファクトチェック・予算順比較表・用途別おすすめ出力 |
| `wantedly-dev-research` | "Wantedlyの求人調べて", "Wantedly開発者採用リサーチ" | Wantedly求人を5クエリ横断検索・7社詳細レポート・AI活用/技術トレンド分析付き |
| `x-dev-research` | "Xのエンジニア採用調べて", "Twitter開発者採用リサーチ" | X(Twitter)上のエンジニア採用ポストを検索・年収データ付き構造化レポート出力 |
| `company-research` | "〇〇社について調べて", "企業リサーチ", "SWOT分析" | 3並列エージェントで12次元の包括的企業分析（概要/MVV/事業/市場/財務/経営陣/3C/強み/ニュース/採用/リスク/SWOT） |

---

## Adding New Plugins

1. Create plugin folder under `plugins/`:
   ```
   plugins/
   └── my-plugin/
       ├── .claude-plugin/
       │   └── plugin.json
       └── skills/
   ```

2. Update `.claude-plugin/marketplace.json`:
   ```json
   {
     "plugins": [
       { "name": "research-tools", "source": "./plugins/research-tools", "enabled": true },
       { "name": "my-plugin", "source": "./plugins/my-plugin", "enabled": true }
     ]
   }
   ```

3. Commit and push (for remote), or run `/plugin marketplace update` (for local).

---

## License

MIT

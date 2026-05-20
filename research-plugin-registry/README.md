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
| research-tools | 1.1.0 | Multi-source web research - restaurant discovery, budget comparison, venue recommendations, X(Twitter) developer recruitment research |

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
| `x-dev-research` | "Xのエンジニア採用調べて", "Twitter開発者採用リサーチ" | X(Twitter)上のエンジニア採用ポストを検索・年収データ付き構造化レポート出力 |

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

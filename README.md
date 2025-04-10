# Dioxide  
*A linter for Go that actually gets you*  

### Why Use Dioxide?  

Most Go linters feel like they were built in 2012. Dioxide doesn’t.  

We built it because:  
- **It’s stupidly thorough** – Finds issues others miss (dead code, circular deps, *even package boundary leaks*)  
- **Fixes your code for you** – Not just nagging – `--fix` actually cleans up 80% of issues automatically  
- **Your rules, not ours** – Hate camelCase? Want 200-character lines? Configure it once and forget  

Think of it like a code janitor that *actually* cleans instead of just pointing out mess.  

---

## What It Does (That Others Don’t)  

| Feature                  | Dioxide | Typical Go Linters |  
|--------------------------|---------|--------------------|  
| Finds unused variables   | ✅       | ✅                  |  
| Detects circular deps    | ✅       | ❌                  |  
| Auto-fix                 | ✅       | ❌                  |  
| Architecture smells      | ✅       | ❌                  |  
| Config-as-code           | ✅       | ❌ (Mostly flags)   |  

---

## Get Started  

### Install (10 seconds)  
```bash  
# Clone & build (you’ll need Rust installed)  
git clone https://github.com/kleeedolinux/dioxide.git  
cd dioxide  
cargo build --release  

# Drop the binary wherever  
cp target/release/dioxide ~/go/bin/  
```

### Basic Use  
```bash  
# Lint your entire project (yes, even vendor/)  
dioxide lint ./  

# Fix what’s fixable  
dioxide lint --fix ./src/  

# Just check one sketchy file  
dioxide lint server.go  
```

---

## Make It Yours  

Create a `dioxide.toml` to:  
- Ignore vendored/generated code  
- Allow 200-character lines (we don’t judge)  
- Disable rules you hate  

```bash  
# Generate default config  
dioxide init  
```

Example config for messy projects:  
```toml  
[general]  
ignore_patterns = ["_test\.go$", "legacy/"]  # Skip tests and legacy dir  
exclude_dirs = ["vendor", "auto_generated"]  

[rules.syntax]  
max_line_length = 200  # Go big or go home  

[rules.dead_code]  
detect_unused_variables = false  # We have... reasons  
```

---

## FAQ  

**Q: Why Rust?**  
A: Because we like speed. Lints 50k LOC in under 2s.  

**Q: Can it replace `golint`/`staticcheck`?**  
A: Yes. Seriously – try it.  

**Q: How’s this different from revive?**  
A: We find architectural issues. And actually fix code.  

---

## Contribute  

Found something Dioxide misses?  
[Open an issue](https://github.com/kleeedolinux/dioxide/issues) – we’ll add it

--- 

*License: MIT (because you deserve to own your tools)*  

---

**P.S.** If you still get style warnings after this – maybe your code actually needs fixing. 😉

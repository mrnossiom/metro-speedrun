# Metro Speedrun

Code to generate graphs of subway networks based on Wikidata queries and find/analyze routes to finish speedrunning paris subway.

# Building

When using the Nix shell, you can expect GLIBC version mismatch from your environment.
Clear your `target/` directory and run cargo build commands inside a isolated dev shell with the following.

```bash
nix develop --ignore-environment

# then, once inside, run the following to avoid clashing with rust-analyzer
export CARGO_TARGET_DIR=/tmp/target-metro-speedrun
mkdir -p CARGO_TARGET_DIR

# and if you wish to just enter a build loop
while :; do cargo build; read; done
```

# Resources

- Sikora, Florian. The Shortest Way to Visit All Metro Lines in a City. 2018, https://arxiv.org/abs/1709.05948.

---

- Beckenbach, Isabel, et al. The S-Bahn Challenge in Berlin. 2015, https://api.semanticscholar.org/CorpusID:114980283. 

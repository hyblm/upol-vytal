//! # Experimentální vyhodnocení
//!
//! > měřeno na ultrabooku Lenovo X1 Carbon 4th gen, s intel Core i5 šesté generace
//!
//! Pomocí nástroje [`hyperfine`](https://github.com/sharkdp/hyperfine) (který každý soubor spouští vícekrát. Alespoň 10 spuštění, nebo celková doba měření alespoň 3 vteřiny),
//! jsem solver spustil na všech souborech z adresáře `benchmarks/100vars/sat`.
//! ```
//! cargo build --release
//! hyperfine --parameter-scan dimacs_file 0 999 -D 1 "./target/release/solver benchmarks/100vars/sat/{dimacs_file}.cnf" --export-csv bench.csv
//! ```
//!
//! Z výsledného souboru můžeme získat průměry naměřených hodnot pomocí `awk`,
//!
//! ```
//! awk -F ',' '{
//!      mean += $2
//!      stddev += $3
//!      n++
//!    }
//!    END {
//!      print mean / n
//!      print stddev / n
//! }' bench.csv
//! ```
//!
//! a pro min a max celého rozsahu všech spuštění jen seřadit.
//!
//! | mean | stddev | min | max |
//! |------|--------|-----|-----|
//! | 50.6969 ms | 1.14666 ms | 644.39236 μs | 278.15658236 ms |
//!
//! # Popis implementace
//!
//! Máme [strukturu s počítadli zachicující statistiky pro výpis](`State`),
//! a [strukturu zachicující uzly rozhodovacího stromu](`Formula`) s [přiřazeními](`Assignment`)
//! a ještě nezplněnými [klauzelemi](`Clause`) v daném uzlu.
//!
//! Průběh výpočtu je popsán ve funkci [`solve`].
//!

use anyhow::{Context, Result};
use std::{env, fs};

/// stejně jako v [DIMACS formátu](https://web.archive.org/web/20190325181937/https://www.satcompetition.org/2009/format-benchmarks2009.html)
/// reprezentován číslem, záporná hodnota značí negaci literálu.
///
type Literal = isize;

/// [Literál](`Literal`) s vyhodnocením na pravdu. Např. hodnota `5` značí, že [literál](`Literal`)
/// `5` se vyhodnotí na pravdu. Hodnota `-3` značí, že [literál](`Literal`) `-3` se
/// vyhodnotí na pravdu, z toho plyne že [literál](`Literal`) `3` se vyhodnotí na nepravdu.
///
pub type Assignment = Literal;

/// Seznam [Literálů](`Literal`)
///
type Clause = Vec<Literal>;

/// Reprezentuje stav prohledávaného podstromu možných [přiřazení](`Assignment`).
/// A to seznamem [přiřazení](`Assignment`), kterými jsme se od původního problému
/// dostali k tomuto stavu, a seznamem [klauzilí](`Clause`) pouze s [literáli](`Literal`),
/// které jsou stále nerozhodnuty.
///
#[derive(Debug, Default, Clone)]
pub struct Formula {
    assignments: Vec<Assignment>,
    clauses: Vec<Clause>,
}

/// Zachicuje počet prozkoumaných uzlů, a počet použití [unit
/// propagace](`Formula::unit_propagate`).
#[derive(Default)]
pub struct State {
    pub unit_propagation_counter: usize,
    pub node_counter: usize,
}

impl Formula {
    /// Vrátí kopii [formule](`Formula`), ve které má daný [literál](`Literal`)
    /// přiřazené pravdivé [vyhodnocení](`Assignment`).
    ///
    fn with_true(&self, literal: Literal) -> Self {
        let mut new = self.clone();
        new.assign_true(literal);
        new
    }

    /// V dané [formuli](`Formula`) přiřadí danému [literálu](`Literal`) pravdivé
    /// [vyhodnocení](`Assignment`).
    ///
    fn assign_true(&mut self, literal: Literal) {
        self.assignments.push(literal);
        self.clauses.retain(|clause| !clause.contains(&literal));

        let inverse = -literal;
        for clause in &mut self.clauses {
            if let Some(i) = clause.iter().position(|lit| lit == &inverse) {
                clause.swap_remove(i);
            }
        }
    }

    /// Najde všechny [klauzule](`Clause`) délky **1** a přiřadí jejich
    /// literálům pravdivé [vyhodnocení](`Assignment`).
    ///
    fn unit_propagate(&mut self, state: &mut State) {
        let mut assign_to_true: Vec<Literal> = self
            .clauses
            .iter()
            .filter(|clause| clause.len() == 1)
            .map(|unit| unit[0])
            .collect();
        assign_to_true.sort();
        assign_to_true.dedup();

        state.unit_propagation_counter += assign_to_true.len();
        for literal in assign_to_true {
            self.assign_true(literal);
        }
    }

    /// Hledá [literál](`Literal`), který se vyskytuje najčastěji v [klauzilých](`Clause`) minimální délky.
    ///
    /// Délkou myslíme počet [literálů](`Literal`) v dané [klauzuli](`Clause`).
    ///
    /// - Zjistí minimální délku [klauzilí](`Cluase`) ve [formuli](`Formula`).
    ///
    /// - Vrátí [literál](`Literal`), který se ve všech [klauzulích](`Clause`) té délky vyskytuje nejčastěji.
    ///
    fn mom(&self) -> Literal {
        let min_len = self
            .clauses
            .iter()
            .map(|clause| clause.len())
            .min()
            .unwrap();

        let mut counts = hashbrown::HashMap::new();
        for shortest_clause in self.clauses.iter().filter(|clause| clause.len() == min_len) {
            for literal in shortest_clause {
                if let Some(value) = counts.get_mut(literal) {
                    *value += 1;
                } else {
                    counts.insert(literal, 0usize);
                }
            }
        }

        let (literal, _) = counts.into_iter().max_by_key(|x| x.1).unwrap();

        *literal
    }
}

/// Pro splnitelný problém, obsahuje seznam [přiřazení](`Assignment`), která daný problém splňují.
///
pub type SatResult = Option<Vec<Assignment>>;

/// Rekurzivně prozkoumává strom možných [přiřazení](`Assignment`). Průběžně jej ořezává pomocí
/// [unit propagace](`Formula::unit_propagate`).
///
/// Postupuje takto:
///
/// 1. Pokusí se ořezat strom k prozkoumání [unit propagací](`Formula::unit_propagate`)
///
/// 2. Zkontroluje, jestli už nemáme splněno (t.j. [seznam klauzilí formule](`Formula`) je prázdný). Pokud ano, vrátí [seznam literálů s pravdivým
///    vyhodnocením](`Assignment`).
///
/// 3. Zkontroluje, jestli existují již nesplintelné klauzule. Pokud ano, ukončíme naše
///    prozkoumávání tohoto podtromu.
///
/// 4. Použije heuristiku [**M**ost **O**ccurences in clauses of **M**inimal length](`Formula::mom`) k výběru
///    ([literálu](`Literal`)) příští větve k prozkoumání.
///
/// 5. Na konec napřed zkoumá podstrom, ve kterém je vybraný [literál](`Literal`) vyhodnocený na pravdu.
///    Když nenajde vyhovující [přiřazení](`Assignment`), pak zkusí zkoumat podstrom, ve kterém je vybraný
///    [literál](`Literal`) vyhodnocený na nepravdu.
///
pub fn solve(state: &mut State, mut formula: Formula) -> SatResult {
    state.node_counter += 1;

    formula.unit_propagate(state);

    if formula.clauses.is_empty() {
        return Some(formula.assignments);
    }
    if formula
        .clauses
        .iter()
        .find(|clause| clause.is_empty())
        .is_some()
    {
        return None;
    }

    let literal = formula.mom();

    return solve(state, formula.with_true(literal))
        .or_else(|| solve(state, formula.with_true(-literal)));
}

/// Převede text [DIMACS formátu](https://web.archive.org/web/20190325181937/https://www.satcompetition.org/2009/format-benchmarks2009.html)
/// do datové struktury [`Formula`].
///
pub fn parse_dimacs_file(problem: &str) -> Result<Formula> {
    let clauses = problem
        .lines()
        .skip_while(|line| line.starts_with('c'))
        .skip(1)
        .into_iter()
        .map(|line| {
            line.split_whitespace()
                .take_while(|str| str != &"0")
                .map(|x| x.parse())
                .collect::<Result<Vec<_>, _>>()
                .with_context(|| format!("Failed to parse line: {}", line))
        })
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to parse dimacs file")?;

    Ok(Formula {
        clauses,
        ..Default::default()
    })
}

/// Načte soubour předaný při spuštění a pokusí se ho načíst pomocí funkce [`parse_dimacs_file`].
///
pub fn process_args() -> Result<Formula> {
    env::args()
        .nth(1)
        .context("No path given")
        .and_then(|path| fs::read_to_string(path).context("failed to read file"))
        .and_then(|problem| parse_dimacs_file(&problem).context("failed to parse problem file"))
}

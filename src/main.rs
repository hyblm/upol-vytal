use anyhow::{Context, Result};
use cpu_time::ProcessTime;
use solver::{process_args, solve, State};

fn main() -> Result<()> {
    let start = ProcessTime::try_now().context("Getting process time failed")?;
    let formula = process_args()?;
    let mut state = State::default();
    let time_init = start.try_elapsed().context("Getting process time failed")?;

    let start = ProcessTime::try_now().context("Getting process time failed")?;
    match solve(&mut state, formula) {
        Some(mut assignments) => {
            assignments.sort();
            println!("SAT");
            println!("true: {:?}", assignments);
        }
        None => println!("UNSAT\n"),
    }
    let time_solution = start.try_elapsed().context("Getting process time failed")?;

    println!(
        "
setup time:        {:#?}
solve time:        {:#?}
unit propagations: {}   
nodes visited:     {}   
",
        time_init, time_solution, state.unit_propagation_counter, state.node_counter
    );
    return Ok(());
}

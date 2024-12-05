use serde::{Deserialize, Serialize};
use satoxid::{CadicalEncoder, constraints::{Or, ExactlyK, Not, And, If}, Encoder, Backend};
use std::{
    path::PathBuf,
    iter,
    fmt::Debug,
    hash::Hash,
};
use clap::{Args, Parser, Subcommand, ValueEnum};
use ron::ser::PrettyConfig;

#[derive(Parser, Debug)]
struct Cli {
    #[arg(long)]
    write_default: bool,
    /// The path to read
    #[arg(required = true, value_name = "FILE", value_hint = clap::ValueHint::FilePath)]
    input: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct Solution {
    note: String,
    pairs: Vec<Pair<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct Input {
    people: Vec<Person>,
    whitelist: Vec<Pair<String>>,
    blacklist: Vec<Pair<String>>,
    blacklist_sets: Vec<Vec<String>>,
    blacklist_symmetric: Vec<Pair<String>>,
    history: Vec<Solution>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
struct Person {
    name: String,
    email: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
struct Pair<T> where T: Debug + Eq + Hash + PartialEq + Clone {
    giver: T,
    receiver: T,
}

impl<T> Pair<T> where T: Debug + Eq + Hash + PartialEq + Clone {
    fn new(x: T, y: T) -> Self {
        Self { giver: x, receiver: y }
    }
}

fn encode_secret_santa_rules<T: Debug + Eq + Hash + PartialEq + Clone>(universe: &[T], encoder: &mut Encoder<Pair<T>, impl Backend>) {
    let len = universe.len();
    // Each person is someone's giver.
    for p in 0..universe.len() {
        // let lits = (0..len).filter_map(|x| (x != p).then_some(Pair { giver: universe[p].clone(), receiver: universe[x].clone() }));
        let lits = (0..len).filter_map(|x| (true).then_some(Pair { giver: universe[p].clone(), receiver: universe[x].clone() }));
        encoder.add_constraint(ExactlyK { k: 1, lits });
    }
    // Each person is someone's receiver.
    for p in 0..universe.len() {
        // let lits = (0..len).filter_map(|x| (x != p).then_some(Pair { giver: universe[x].clone(), receiver: universe[p].clone() }));
        let lits = (0..len).filter_map(|x| (true).then_some(Pair { giver: universe[x].clone(), receiver: universe[p].clone() }));
        encoder.add_constraint(ExactlyK { k: 1, lits });
    }

    // No one can give to themselves.
    let lits = (0..len).map(|p| Pair { giver: universe[p].clone(), receiver: universe[p].clone() });
    encoder.add_constraint(Not(Or(lits)));

    // Don't have small cycles.
    for p in 0..universe.len() {
        for j in p..universe.len() {
            encoder.add_constraint(If { cond: Pair { giver: universe[p].clone(), receiver: universe[j].clone() },
                                        then: Not(Pair { giver: universe[j].clone(), receiver: universe[p].clone() }) });

        }
    }
}

fn exclude_pairs<T: Debug + Eq + Hash + PartialEq + Clone>(lits: impl Iterator<Item = Pair<T>> + Clone,
                                                           encoder: &mut Encoder<Pair<T>, impl Backend>) {
    encoder.add_constraint(Not(Or(lits)));
}

fn exclude_pairs_symmetric<T: Debug + Eq + Hash + PartialEq + Clone>(lits: impl Iterator<Item = Pair<T>> + Clone,
                                                                     encoder: &mut Encoder<Pair<T>, impl Backend>) {
    exclude_pairs(lits.clone(), encoder);
    exclude_pairs(lits.map(|Pair { giver: a, receiver: b }| Pair { giver: b, receiver: a }), encoder);
}

fn exclude_sets<T: Debug + Eq + Hash + PartialEq + Clone>(people: Vec<T>,
                                                          encoder: &mut Encoder<Pair<T>, impl Backend>) {
    let len = people.len();
    let mut accum = vec![];
    for x in 0..len {
        for y in x..len {
            accum.push(Pair { giver: people[x].clone(), receiver: people[y].clone() });
        }
    }
    exclude_pairs_symmetric(accum.into_iter(), encoder);
}

fn main() {
    let cli = Cli::parse();

    if cli.write_default {
        let mut input = Input::default();
        let a = Person { name: String::from("John"), email: String::from("john@email.com") };
        let b = Person { name: String::from("Sean"), email: String::from("sean@email.com") };
        let c = Person { name: String::from("Shane"), email: String::from("shane@email.com") };
        input.people.push(a.clone());
        input.people.push(b.clone());
        input.people.push(c.clone());
        input.blacklist_sets.push(vec![a.name.clone(), b.name.clone()]);
        input.whitelist.push(Pair::new(b.name.clone(), c.name.clone()));
        input.blacklist.push(Pair::new(b.name.clone(), c.name.clone()));
        input.blacklist_symmetric.push(Pair::new(b.name.clone(), c.name.clone()));
        input.history.push(Solution { note: "Year 2024".into(), pairs: vec![Pair::new(a.name.clone(), c.name.clone()),
                                                                            Pair::new(b.name.clone(), a.name.clone()),
                                                                            Pair::new(c.name.clone(), b.name.clone())]});
        // TODO: Should use a stream here.
        println!("{}", ron::ser::to_string_pretty(&input, PrettyConfig::default()).unwrap());
        return;
    }

    let mut encoder = CadicalEncoder::new();

    let people: Vec<u8> = (0..4).collect();

    encoder.add_constraint(Or(iter::once(Pair { giver: 0u8, receiver: 1u8 })));

    encode_secret_santa_rules(&people, &mut encoder);

    if let Some(model) = encoder.solve() {

        for var in model.vars() {
            // println!("{:?} {}", var, var.is_pos());
            if var.is_pos() {
                println!("{:?}", var);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ron::ser::PrettyConfig;

    fn p0() -> Person {
        Person { name: String::from("First Last"), email: String::from("name@email.com") }
    }


    #[test]
    fn parse_person() {
        let p = p0();
        // assert_eq!(ron::ser::to_string_pretty(&p, PrettyConfig::default()).unwrap(), "");
        assert_eq!(ron::ser::to_string(&p).unwrap(), "(name:\"First Last\",email:\"name@email.com\")");
    }
}

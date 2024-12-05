use serde::{Deserialize, Serialize};
use satoxid::{CadicalEncoder, constraints::{Or, ExactlyK, Not, And, If}, Encoder, Backend};
use std::{
    cmp::Reverse,
    fmt::{self, Debug, Write},
    fs::File,
    path::PathBuf,
    iter,
    hash::Hash,
};
use clap::{Args, Parser, Subcommand, ValueEnum};
use ron::ser::PrettyConfig;
use cmd_lib::run_cmd;

#[derive(Parser, Debug)]
struct Cli {
    #[arg(long)]
    write_default: bool,
    /// Execute a command: cat $body | $exec -s "$subject" "First <name@email.com>"
    #[arg(long)]
    exec: Option<String>,
    /// Dry run, won't execute command, will echo it: cat $body; echo $exec -s "$subject" "First <name@email.com>"
    #[arg(long, short = 'n', default_value_t = false)]
    dry_run: bool,
    /// The path to read
    #[arg(required = true, value_name = "FILE", value_hint = clap::ValueHint::FilePath)]
    input: PathBuf,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct Solution {
    year: u16,
    exclude_pairs: bool,
    pairs: Vec<Pair<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct Input {
    people: Vec<Person>,
    whitelist: Vec<Pair<String>>,
    blacklist: Vec<Pair<String>>,
    blacklist_sets: Vec<Vec<String>>,
    history: Vec<Solution>,
}

impl Input {
    /// Confirm all names present are in the people list.
    fn check_history(&self) {
        for solution in &self.history {
            for pair in &solution.pairs {
                if self.people.iter().find(|p| p.name == pair.giver).is_none() {
                    panic!("Giver named '{}' present in history but not found in people set.", &pair.giver);
                }

                if self.people.iter().find(|p| p.name == pair.receiver).is_none() {
                    panic!("Receiver named '{}' present in history but not found in people set.", &pair.receiver);
                }
            }
        }
    }
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

fn include_pairs<T: Debug + Eq + Hash + PartialEq + Clone>(lits: impl Iterator<Item = Pair<T>> + Clone,
                                                           encoder: &mut Encoder<Pair<T>, impl Backend>) {
    encoder.add_constraint(And(lits));
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

fn exclude_sets<T: Debug + Eq + Hash + PartialEq + Clone>(people: &[T],
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

#[derive(Debug)]
struct Message {
    subject: String,
    body: String,
    email: String,
}

/// Return the givers for this person.
fn givers_for<'a>(receiver: &'a str, input: &'a Input) -> impl Iterator<Item = String> + use<'a> {
    input.history.iter().flat_map(move |x| x.pairs.iter().filter(move |p| p.receiver == receiver).map(|p|
                                                                                                      format!("{} ({})", p.giver, x.year)))
}

/// Return the receivers for this person.
fn receivers_for<'a>(giver: &'a str, input: &'a Input) -> impl Iterator<Item = String> + use<'a> {
    input.history.iter().flat_map(move |x| x.pairs.iter().filter(move |p| p.giver == giver).map(|p|
                                                                                                format!("{} ({})", p.receiver, x.year)))
}

fn compose_message(pair: &Pair<String>, input: &Input) -> Result<Message, fmt::Error> {
    let giver = &pair.giver;
    let receiver = &pair.receiver;
    let subject = format!("Secret Santa {giver}: Keep it secret! Keep it safe!");
    let mut body = String::new();
    writeln!(body, "{giver}, you are the Secret Santa for {receiver}.")?;

    let mut receivers = receivers_for(giver, input).peekable();

    if receivers.peek().is_some() {
        writeln!(body, "")?;
        write!(body, "You were Secret Santa for ")?;
        write!(body, "{}", receivers.next().unwrap())?;
        while let Some(receiver) = receivers.next() {
            if receivers.peek().is_none() {
                write!(body, ", and {}", receiver)?;
            } else {
                write!(body, ", {}", receiver)?;
            }
        }
        writeln!(body, ".")?;
    }

    let mut givers = givers_for(giver, input).peekable();

    if givers.peek().is_some() {
        writeln!(body, "")?;
        write!(body, "You had these Secret Santas in Christmases past: ")?;
        write!(body, "{}", givers.next().unwrap())?;
        // for giver in givers {
        //     write!(body, ", {}", giver);
        // }
        while let Some(giver) = givers.next() {
            if givers.peek().is_none() {
                write!(body, ", and {}", giver)?;
            } else {
                write!(body, ", {}", giver)?;
            }
        }
        writeln!(body, ".")?;
    }

    writeln!(body, r#"
* * *
Brought to you by secret-santa[1].

[1]: https://github.com/shanecelis/secret-santa/blob/main/src/main.rs
"#)?;

    let email = input.people.iter().find(|p| p.name == pair.giver).expect("Failed to find email address").email.clone();
    let name_and_email = format!("{} <{}>", pair.giver, email);
    Ok(Message { subject, body, email: name_and_email })
}

fn main() -> std::io::Result<()>{
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
        input.history.push(Solution { year: 2024, exclude_pairs: true,
                                      pairs: vec![Pair::new(a.name.clone(), c.name.clone()),
                                                  Pair::new(b.name.clone(), a.name.clone()),
                                                  Pair::new(c.name.clone(), b.name.clone())]});
        // TODO: Should use a stream here.
        println!("{}", ron::ser::to_string_pretty(&input, PrettyConfig::default()).unwrap());
        return Ok(());
    }

    let f = File::open(cli.input).expect("Failed opening");

    let mut input: Input = ron::de::from_reader(f).expect("Failed parsing");
    input.check_history();

    let mut encoder = CadicalEncoder::new();
    input.history.sort_by_key(|sol| Reverse(sol.year));
    let names: Vec<String> = input.people.iter().map(|p| p.name.clone()).collect();
    encode_secret_santa_rules(&names, &mut encoder);
    for blacklist_set in &input.blacklist_sets {
        exclude_sets(blacklist_set, &mut encoder);
    }
    exclude_pairs(input.blacklist.iter().cloned(), &mut encoder);
    include_pairs(input.whitelist.iter().cloned(), &mut encoder);

    // Exclude historical pairs.
    for solution in &input.history {
        if ! solution.exclude_pairs {
            continue;
        }
        exclude_pairs(solution.pairs.iter().cloned(), &mut encoder);
    }

    if let Some(model) = encoder.solve() {

        let mut pairs: Vec<Pair<String>> = model.vars().filter_map(|v| v.is_pos().then(|| v.unwrap())).collect();
        pairs.sort_by_key(|p| p.giver.clone());

        let mut msgs = vec![];
        /// Generate all the messages first to confirm there aren't any errors.
        for pair in &pairs {
            println!("{:?}", pair);
            let msg = compose_message(pair, &input).expect("Failed to compose message");
            msgs.push(msg);
        }

        for msg in msgs {
            let subject = msg.subject;
            let body = msg.body;
            let email = msg.email;
            let arg = r"1 2 3";

            if let Some(ref exec) = cli.exec {
                let exec_args: Vec<&str> = exec.split_whitespace().collect();

                if cli.dry_run {
                    run_cmd!(echo $body | cat; echo $[exec_args] -s $subject $email)?;
                } else {
                    run_cmd!(echo $body | $[exec_args] -s $subject $email)?;
                }
            }
        }
    } else {
        eprintln!("No secret santa solutions found!");
        std::process::exit(1);
    }
    Ok(())
}


#[cfg(test)]
mod test {
    use super::*;


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

// TODO: Add these subcommands next year.
//
// #[derive(Debug, Subcommand)]
// enum Subcommands {
//     /// Write creature to file path
//     #[command(arg_required_else_help = true)]
//     Generate {
//         #[arg(required = true, value_name = "FILE", value_hint = clap::ValueHint::FilePath)]
//         input: PathBuf,
//         /// The path to write
//         #[arg(required = true, value_name = "FILE", value_hint = clap::ValueHint::FilePath)]
//         output: PathBuf,
//     },
//     Execute {
//         #[arg(required = true, value_name = "FILE", value_hint = clap::ValueHint::FilePath)]
//         input: PathBuf,
//         /// The path to write
//         #[arg(required = true, value_name = "FILE", value_hint = clap::ValueHint::FilePath)]
//         output: PathBuf,
//     },
//     #[command(arg_required_else_help = true)]
//     Read {
//         /// The path to read
//         #[arg(required = true, value_name = "FILE", value_hint = clap::ValueHint::FilePath)]
//         path: PathBuf,
//     },
// }

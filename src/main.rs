use satoxid::{CadicalEncoder, constraints::{Or, ExactlyK, Not, And}, Encoder, Backend};
use std::{
    iter,
    fmt::Debug,
    hash::Hash,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Person<T>(T);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Pair<T> where T: Debug + Eq + Hash + PartialEq + Clone {
    giver: T,
    receiver: T,
}

fn encode_secret_santa_rules<T: Debug + Eq + Hash + PartialEq + Clone>(universe: &[T], encoder: &mut Encoder<Pair<T>, impl Backend>) {
    let len = universe.len();
    // Each person is someone's santa.
    for p in 0..universe.len() {
        // let lits = (0..len).filter_map(|x| (x != p).then_some(Pair { giver: universe[p].clone(), receiver: universe[x].clone() }));
        let lits = (0..len).filter_map(|x| (true).then_some(Pair { giver: universe[p].clone(), receiver: universe[x].clone() }));
        encoder.add_constraint(ExactlyK { k: 1, lits });
    }
    for p in 0..universe.len() {
        // let lits = (0..len).filter_map(|x| (x != p).then_some(Pair { giver: universe[x].clone(), receiver: universe[p].clone() }));
        let lits = (0..len).filter_map(|x| (true).then_some(Pair { giver: universe[x].clone(), receiver: universe[p].clone() }));
        encoder.add_constraint(ExactlyK { k: 1, lits });
    }

    /// No one can give to themselves.
    let lits = (0..len).map(|p| Pair { giver: universe[p].clone(), receiver: universe[p].clone() });
    encoder.add_constraint(Not(Or(lits)));
}

fn exclude_pairs<T: Debug + Eq + Hash + PartialEq + Clone>(iter: impl Iterator<Item = Pair<T>>, encoder: &mut Encoder<Pair<T>, impl Backend>) {
    encoder.add_constraint(Not(Or(lits)));
}

fn main() {
    let mut encoder = CadicalEncoder::new();

    let people: Vec<u8> = (0..3).collect();

    encoder.add_constraint(Or(iter::once(Pair { giver: 0u8, receiver: 1u8 })));

    // let constraint = ExactlyK {
    //     k: 1,
    //     lits: [A, B, C].iter().copied()
    // };

    encode_secret_santa_rules(&people, &mut encoder);
    // encoder.add_constraint(constraint);

    if let Some(model) = encoder.solve() {

        for var in model.vars() {
            // println!("{:?} {}", var, var.is_pos());
            if var.is_pos() {
                println!("{:?}", var);
            }
        }
    }
}

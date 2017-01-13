use permutation;
use std::collections::BTreeSet;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::mpsc;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Subset {
    // The number of objects the permutations act on.
    size : usize,
    // The permutations in the set.
    elements : BTreeSet<permutation::Permutation>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Subgroup(Subset);
#[derive(Debug)]
pub struct ConjugacyClass(Subset);

fn subset_size(elements : &BTreeSet<permutation::Permutation>) -> Option<usize> {
    // TODO: we wanted to just take "an element" from the elements
    // but that requires iterators which take a mutable reference,
    // and then this happened.
    let mut size : Option<usize>= None;
    for elem in elements {
        match size {
            None => {
                //size = Some(elem.permutation.len());
                return Some(elem.permutation.len());
            }
            Some(expected_size) => {
                if elem.permutation.len() != expected_size {
                    return None;
                }
            }
        }
    };
    size
}

fn check_closed(subset : Subset) -> Option<Subgroup> {
    // We need to check the group is closed under all operations.
    /*
    {
        let elements = &(subset.elements);
        for g in elements {
            for h in elements {
                let tempelem = permutation::composition(g,&permutation::invert(&h));
                if !elements.contains(&tempelem) {
                    return None;
                }
            }
        }
    };
    */

    Some(Subgroup(subset))
}

pub fn make_subset(elements : BTreeSet<permutation::Permutation>) -> Option<Subset> {
    subset_size(&elements)
        .map(|size| Subset { size: size, elements: elements })
}

// When the set is a group, wrap it in the Subgroup type.
pub fn make_subgroup(elements : BTreeSet<permutation::Permutation>) -> Option<Subgroup> {
    // First we make it into a Subset by checking the sizes.
    // For this, we need at least one element.
    make_subset(elements)
        .and_then(|subset| check_closed(subset))

}

// Generate the trivial group on the given number of elements.
pub fn trivial(size : usize) -> Subgroup {
    let mut group = BTreeSet::new();
    group.insert(permutation::identity(size));
    make_subgroup(group).unwrap()
}

pub fn conjugate( subgroup : &Subgroup, g : &permutation::Permutation) -> Subgroup {
    let mut newgroup = BTreeSet::new();
    let Subgroup(ref elem_set) = *subgroup;
    for elem in &elem_set.elements {
        let conjugate_elem = permutation::composition(g,&permutation::composition(elem,&permutation::invert(g)));
        newgroup.insert(conjugate_elem);
    }
    make_subgroup(newgroup).unwrap()
}

/*
// This implementation trades space efficiency for time efficiency,
// it turns out my memory is too limited to actually make this tradeoff.
pub fn generate(generators : &Subset) -> Subgroup {
    let mut result = BTreeSet::new();
    {
        let mut to_visit = VecDeque::new();
        result.insert(permutation::identity(generators.size));
        for elem1 in &generators.elements {
            for elem2 in &generators.elements {
                to_visit.push_back((elem1.clone(), elem2.clone()));
            }
            result.insert(elem1.clone());
        }
        while let Some((elem1, elem2)) = to_visit.pop_front() {
            let product = permutation::composition(&elem1, &elem2);
            if result.insert(product.clone()) {
                // since we move product to the set, we have to know where it
                // ends up
                for elem1 in &result {
                    to_visit.push_back((elem1.clone(), product.clone()));
                    to_visit.push_back((product.clone(), elem1.clone()));
                }
            }
        }
    }
    make_subgroup(result).unwrap()
}
*/

// Calculate the group generated by these permutations,
// by taking the product of all elements we have generated up to now repeatedly.
pub fn generate_fixpoint(generators : &Subset) -> Subgroup {
    let mut old_result = BTreeSet::new();
    let mut new_result = BTreeSet::new();
    for elem1 in &generators.elements {
        new_result.insert(elem1.clone());
    }
    while old_result != new_result {
        old_result = new_result.clone();
        for elem1 in &old_result {
            for elem2 in &old_result {
                new_result.insert(permutation::composition(elem1, elem2));
            }
        }
    }
    make_subgroup(new_result).unwrap()
}

// calculate all elements of the symmetric group S_n (for n > 1)
pub fn elements(size : usize) -> Subgroup {
    assert!(size > 1);

    let mut cycle = (2..size+1).collect::<Vec<usize>>();
    cycle.push(1);
    let mut transposition = vec![2, 1];
    transposition.extend(3..size+1);

    let gen1 = permutation::make_permutation(cycle).unwrap();
    let gen2 = permutation::make_permutation(transposition).unwrap();
    let generators = make_subset([gen1, gen2].iter().cloned().collect()).unwrap();
    generate_fixpoint(&generators)
}

pub fn all_subgroups(size : usize) -> BTreeSet<Subgroup> {
    let Subgroup(elem_set) = elements(size);
    let elems = elem_set.elements;

    // We'll write our results into this set
    let mut result = Arc::new(Mutex::new(BTreeSet::new()));
    // Use the channel to notify the main thread when we're done.
    let (tx, rx) = mpsc::channel();
    let mut channels = Vec::new();

    let threadCount = 8;
    for i in 0 .. threadCount {
        let (tx, rx) = mspc::channel();
        channels.append(tx);

        let resultCell = result.clone();
        thread::spawn(move || {
            // As long as there is work to do, do work.
            for (elem1, elem2, elem3) in rx {
                // TODO: dit kan beter.
                let generators = make_subset([elem1, elem2, elem3].iter()
                    .cloned().collect()).unwrap();

                let mut resultRef = resultCell.lock().unwrap();
                resultRef.insert(generate_fixpoint(&generators));
            }
            // Notify that we're finished.
            tx.send(()).unwrap();
        });
    }

    let mut elemCount = 0;
    for elem1 in &elems {
        for elem2 in &elems {
            for elem3 in &elems {
                channels[elemCount % threadCount].send((elem1, elem2, elem3)).unwrap();
                elemCount++;
            }
        }
    }

    // tell the threads that this is everything
    drop(channels);
    for _ in 0..threadCount {
        // Wait for threads to finish.
        rx.recv().unwrap();
    }

    let mut resultRef = result.lock().unwrap();
    resultRef.clone()
}

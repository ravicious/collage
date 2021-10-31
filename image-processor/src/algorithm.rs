use genevo::{
    genetic::{Children, Parents},
    operator::{prelude::*, CrossoverOp, GeneticOperator, MutationOp},
    prelude::*,
    random::Rng,
};
use image::RgbImage;
use std::cmp::Ordering;

use crate::layout::{Layout, LayoutNode};

// Phenotype is layout node.
// Genotype is layout.

#[derive(Clone, Debug)]
struct FitnessCalc;

impl<'a> Genotype for Layout<'a> {
    type Dna = LayoutNode<'a>;
}

// By default, genevo (the lib for genetic algorithms) assumes that the greater the fitness value
// the better. It also doesn't work with floats. Our cost function returns a float and the lower
// value the better.
//
// Thus we need a wrapper for f64 which implements Ord (f64 implements only PartialOrd) and
// reverses the ordering.
#[derive(Copy, Clone, Debug)]
struct FitnessFloat(f64);

impl PartialEq for FitnessFloat {
    fn eq(&self, other: &Self) -> bool {
        self.0.total_cmp(&other.0) == Ordering::Equal
    }
}

impl Eq for FitnessFloat {}

impl PartialOrd for FitnessFloat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FitnessFloat {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.0.total_cmp(&other.0) {
            Ordering::Greater => Ordering::Less,
            Ordering::Equal => Ordering::Equal,
            Ordering::Less => Ordering::Greater,
        }
    }
}

impl Fitness for FitnessFloat {
    fn zero() -> Self {
        FitnessFloat(0.0)
    }

    fn abs_diff(&self, other: &Self) -> Self {
        let diff = self.0 - other.0;
        FitnessFloat(diff.abs())
    }
}

impl FitnessFunction<Layout<'_>, FitnessFloat> for FitnessCalc {
    fn fitness_of(&self, layout: &Layout) -> FitnessFloat {
        FitnessFloat(layout.cost())
    }

    fn average(&self, fitness_values: &[FitnessFloat]) -> FitnessFloat {
        FitnessFloat(fitness_values.iter().map(|v| v.0).sum::<f64>() / fitness_values.len() as f64)
    }

    fn highest_possible_fitness(&self) -> FitnessFloat {
        FitnessFloat::zero()
    }

    fn lowest_possible_fitness(&self) -> FitnessFloat {
        FitnessFloat(f64::INFINITY)
    }
}

#[derive(Debug, Clone)]
struct LayoutCrossover;

impl LayoutCrossover {
    pub fn new() -> Self {
        LayoutCrossover {}
    }
}

impl GeneticOperator for LayoutCrossover {
    fn name() -> String {
        "Layout-Crossover".to_string()
    }
}

impl<'a> CrossoverOp<Layout<'a>> for LayoutCrossover {
    fn crossover<R>(&self, parents: Parents<Layout<'a>>, _: &mut R) -> Children<Layout<'a>>
    where
        R: Rng + Sized,
    {
        // TODO: Implement actual crossover.
        parents.clone()
    }
}

#[derive(Debug, Clone)]
struct LayoutMutation;

impl LayoutMutation {
    pub fn new() -> Self {
        LayoutMutation {}
    }
}

impl GeneticOperator for LayoutMutation {
    fn name() -> String {
        "Layout-Mutation".to_string()
    }
}

impl<'a> MutationOp<Layout<'a>> for LayoutMutation {
    fn mutate<R>(&self, genome: Layout<'a>, rng: &mut R) -> Layout<'a>
    where
        R: Rng + Sized,
    {
        let mut mutated = genome.clone();

        match rng.gen_range(0, 3) {
            0 => {
                mutated.swap_random_node_pair(rng);
            }
            1 => {
                mutated.randomize_width(rng);
            }
            2 => {
                mutated.randomize_height(rng);
            }
            _ => {
                unreachable!();
            }
        }

        mutated
    }
}

pub fn generate_layout(images: &[RgbImage]) -> Result<Layout, String> {
    // 49 is the max population size. For values bigger than that, genevo starts parallelizing the
    // work by using rayon, which doesn't work OOTB for the Wasm target.
    let population_size = 49;
    let generation_limit = if cfg!(debug_assertions) { 200 } else { 4_000 };
    let selection_ratio = 0.7;
    let num_individuals_per_parents = 2;
    let reinsertion_ratio = 0.7;
    // End of genevo params.

    let initial_population =
        Population::with_individuals((0..population_size).map(|_| Layout::new(images)).collect());

    let mut layout_sim = simulate(
        genetic_algorithm()
            .with_evaluation(FitnessCalc)
            .with_selection(MaximizeSelector::new(
                selection_ratio,
                num_individuals_per_parents,
            ))
            .with_crossover(LayoutCrossover::new())
            .with_mutation(LayoutMutation::new())
            .with_reinsertion(ElitistReinserter::new(FitnessCalc, true, reinsertion_ratio))
            .with_initial_population(initial_population)
            .build(),
    )
    .until(or(
        FitnessLimit::new(FitnessCalc.highest_possible_fitness()),
        GenerationLimit::new(generation_limit),
    ))
    .build();

    if let Ok(SimResult::Final(step, _, _, _)) = layout_sim.run() {
        return Ok(step.result.best_solution.solution.genome);
    }

    Err("something went wrong with layout_sim.run()".to_string())
}

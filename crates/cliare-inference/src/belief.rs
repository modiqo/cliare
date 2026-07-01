#[derive(Debug, Clone)]
pub struct Belief {
    log_odds: f64,
}

impl Belief {
    pub fn with_prior(probability: f64) -> Self {
        let probability = probability.clamp(0.001, 0.999);
        Self {
            log_odds: (probability / (1.0 - probability)).ln(),
        }
    }

    pub fn update(&mut self, weight: f64) {
        self.log_odds += weight;
    }

    pub fn probability(&self) -> f64 {
        1.0 / (1.0 + (-self.log_odds).exp())
    }
}

#[cfg(test)]
mod tests {
    use super::Belief;

    #[test]
    fn positive_evidence_increases_probability() {
        let mut belief = Belief::with_prior(0.25);
        let before = belief.probability();

        belief.update(1.5);

        assert!(belief.probability() > before);
    }
}

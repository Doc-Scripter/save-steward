use crate::detection::{GameCandidate, DetectionError};

#[derive(Debug, Clone)]
pub struct ConfidenceScore {
    pub total_score: f32,
    pub evidence_count: u32,
    pub high_confidence_evidence: u32,
    pub medium_confidence_evidence: u32,
    pub low_confidence_evidence: u32,
    pub conflicting_factors: Vec<String>,
    pub is_reliable: bool,
}

#[derive(Debug, Clone)]
pub struct ConfidenceScorer {
    platform_weights: PlatformWeights,
    evidence_weights: EvidenceWeights,
}

impl ConfidenceScorer {
    pub fn new() -> Self {
        Self {
            platform_weights: PlatformWeights::default(),
            evidence_weights: EvidenceWeights::default(),
        }
    }

    pub fn calculate_overall_confidence(&self, candidates: &[GameCandidate]) -> (f32, bool, Option<String>) {
        if candidates.is_empty() {
            return (0.0, false, Some("No candidates found".to_string()));
        }

        if candidates.len() == 1 {
            let candidate = &candidates[0];
            let confidence = self.calculate_single_candidate_confidence(candidate);
            let needs_confirmation = confidence.total_score < 70.0;
            (confidence.total_score, needs_confirmation, None)
        } else {
            self.calculate_multiple_candidates_confidence(candidates)
        }
    }

    fn calculate_single_candidate_confidence(&self, candidate: &GameCandidate) -> ConfidenceScore {
        let mut total_score = candidate.confidence_score;
        let mut evidence_count = 1;
        let mut high_confidence_evidence = 0;
        let mut medium_confidence_evidence = 0;
        let mut low_confidence_evidence = 0;
        let mut conflicting_factors = Vec::new();

        // Boost score based on platform
        if let Some(platform) = &candidate.platform {
            let platform_boost = self.platform_weights.get_boost(platform);
            total_score += platform_boost;
            evidence_count += 1;

            if platform_boost > 15.0 {
                high_confidence_evidence += 1;
            } else if platform_boost > 5.0 {
                medium_confidence_evidence += 1;
            } else {
                low_confidence_evidence += 1;
            }
        }

        // Boost score based on number of matching identifiers
        let identifier_boost = (candidate.matched_identifiers.len() as f32) * 5.0;
        total_score += identifier_boost.min(25.0); // Cap at 25 points
        evidence_count += candidate.matched_identifiers.len() as u32;

        // Categorize evidence based on matched identifiers
        for identifier in &candidate.matched_identifiers {
            match identifier.as_str() {
                id if id.contains("executable_hash") => high_confidence_evidence += 1,
                id if id.contains("steam") || id.contains("epic") || id.contains("gog") => high_confidence_evidence += 1,
                id if id.contains("process_name") => medium_confidence_evidence += 1,
                id if id.contains("window_title") => medium_confidence_evidence += 1,
                _ => low_confidence_evidence += 1,
            }
        }

        // Check for signs of unreliability
        if total_score < 30.0 {
            conflicting_factors.push("Low overall confidence".to_string());
        }

        if candidate.matched_identifiers.is_empty() {
            conflicting_factors.push("No direct identifier matches".to_string());
        }

        if candidate.platform.is_none() {
            conflicting_factors.push("No platform information available".to_string());
        }

        // Cap final score
        let final_score = total_score.min(100.0).max(0.0);
        let is_reliable = high_confidence_evidence > 0 ||
                         (medium_confidence_evidence >= 2 && final_score >= 60.0);

        ConfidenceScore {
            total_score: final_score,
            evidence_count,
            high_confidence_evidence,
            medium_confidence_evidence,
            low_confidence_evidence,
            conflicting_factors,
            is_reliable,
        }
    }

    fn calculate_multiple_candidates_confidence(&self, candidates: &[GameCandidate]) -> (f32, bool, Option<String>) {
        // Find the best candidate
        let best_candidate = candidates.iter()
            .max_by(|a, b| {
                let a_score = self.calculate_single_candidate_confidence(a).total_score;
                let b_score = self.calculate_single_candidate_confidence(b).total_score;
                a_score.partial_cmp(&b_score).unwrap_or(std::cmp::Ordering::Equal)
            });

        if let Some(best) = best_candidate {
            let best_confidence = self.calculate_single_candidate_confidence(best);

            // Check for close competition
            let second_best = candidates.iter()
                .filter(|c| c.game_id != best.game_id)
                .max_by(|a, b| {
                    let a_score = self.calculate_single_candidate_confidence(a).total_score;
                    let b_score = self.calculate_single_candidate_confidence(b).total_score;
                    a_score.partial_cmp(&b_score).unwrap_or(std::cmp::Ordering::Equal)
                });

            if let Some(second) = second_best {
                let second_confidence = self.calculate_single_candidate_confidence(second);
                let score_difference = best_confidence.total_score - second_confidence.total_score;

                if score_difference < 10.0 {
                    // Close competition - recommend manual confirmation
                    let conflict_reason = format!(
                        "Multiple candidates with similar confidence: {} ({}%) vs {} ({}%)",
                        best.name, best_confidence.total_score,
                        second.name, second_confidence.total_score
                    );
                    return (best_confidence.total_score, true, Some(conflict_reason));
                }
            }

            // Clear winner found
            (best_confidence.total_score, best_confidence.total_score < 75.0, None)
        } else {
            (0.0, false, Some("No valid candidates found".to_string()))
        }
    }

    pub fn validate_identification(&self, candidates: &[GameCandidate], selected_game_id: i64) -> Result<ValidationResult, DetectionError> {
        let selected_candidate = candidates.iter()
            .find(|c| c.game_id == selected_game_id)
            .ok_or_else(|| DetectionError::ConflictError(format!("Selected game ID {} not found in candidates", selected_game_id)))?;

        let confidence = self.calculate_single_candidate_confidence(selected_candidate);

        // Check for red flags
        let mut red_flags = Vec::new();

        if confidence.total_score < 50.0 {
            red_flags.push("Very low confidence score".to_string());
        }

        if confidence.high_confidence_evidence == 0 && confidence.medium_confidence_evidence < 2 {
            red_flags.push("Insufficient strong evidence".to_string());
        }

        if candidates.len() > 3 {
            red_flags.push("Many conflicting candidates".to_string());
        }

        let should_warn = !red_flags.is_empty();
        let is_confirmed = confidence.is_reliable && !should_warn;

        Ok(ValidationResult {
            is_valid: confidence.total_score > 30.0,
            confidence_score: confidence.total_score,
            should_warn,
            should_confirm: !is_confirmed,
            red_flags,
        })
    }

    pub fn explain_confidence_score(&self, candidates: &[GameCandidate], selected_game_id: Option<i64>) -> String {
        if candidates.is_empty() {
            return "No games found that match the provided criteria.".to_string();
        }

        let mut explanation = String::new();

        if let Some(game_id) = selected_game_id {
            if let Some(candidate) = candidates.iter().find(|c| c.game_id == game_id) {
                let confidence = self.calculate_single_candidate_confidence(candidate);

                explanation.push_str(&format!(
                    "Selected '{}' with {:.1}% confidence based on {} pieces of evidence.\n",
                    candidate.name, confidence.total_score, confidence.evidence_count
                ));

                if confidence.high_confidence_evidence > 0 {
                    explanation.push_str(&format!("• {} high-confidence matches\n", confidence.high_confidence_evidence));
                }
                if confidence.medium_confidence_evidence > 0 {
                    explanation.push_str(&format!("• {} medium-confidence matches\n", confidence.medium_confidence_evidence));
                }
                if confidence.low_confidence_evidence > 0 {
                    explanation.push_str(&format!("• {} low-confidence matches\n", confidence.low_confidence_evidence));
                }

                if !confidence.conflicting_factors.is_empty() {
                    explanation.push_str(&format!("\nNote: {}\n", confidence.conflicting_factors.join(", ")));
                }
            }
        } else {
            explanation.push_str(&format!("Found {} potential games:\n", candidates.len()));

            for (i, candidate) in candidates.iter().take(3).enumerate() {
                let confidence = self.calculate_single_candidate_confidence(candidate);
                explanation.push_str(&format!(
                    "{}. {} - {:.1}% confidence\n",
                    i + 1, candidate.name, confidence.total_score
                ));
            }

            if candidates.len() > 3 {
                explanation.push_str(&format!("... and {} more\n", candidates.len() - 3));
            }
        }

        explanation
    }
}

#[derive(Debug, Clone)]
struct PlatformWeights {
    weights: std::collections::HashMap<String, f32>,
}

impl PlatformWeights {
    fn default() -> Self {
        let mut weights = std::collections::HashMap::new();
        weights.insert("steam".to_string(), 15.0);    // Very reliable
        weights.insert("epic".to_string(), 12.0);     // Reliable
        weights.insert("gog".to_string(), 12.0);      // Reliable
        weights.insert("origin".to_string(), 8.0);    // Less reliable
        weights.insert("uplay".to_string(), 8.0);     // Less reliable
        weights.insert("standalone".to_string(), 0.0); // Neutral

        Self { weights }
    }

    fn get_boost(&self, platform: &str) -> f32 {
        self.weights.get(platform).copied().unwrap_or(0.0)
    }
}

#[derive(Debug, Clone)]
struct EvidenceWeights {
    high_confidence: Vec<String>,
    medium_confidence: Vec<String>,
    low_confidence: Vec<String>,
}

impl EvidenceWeights {
    fn default() -> Self {
        Self {
            high_confidence: vec![
                "executable_hash".to_string(),
                "steam_appid".to_string(),
                "epic_id".to_string(),
                "gog_id".to_string(),
                "digital_signature".to_string(),
            ],
            medium_confidence: vec![
                "process_name".to_string(),
                "window_title".to_string(),
                "file_size".to_string(),
                "company_name".to_string(),
            ],
            low_confidence: vec![
                "directory_name".to_string(),
                "file_pattern".to_string(),
                "partial_match".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub confidence_score: f32,
    pub should_warn: bool,
    pub should_confirm: bool,
    pub red_flags: Vec<String>,
}

impl Default for ConfidenceScorer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_candidate(game_id: i64, name: &str, platform: &str, confidence: f32) -> GameCandidate {
        GameCandidate {
            game_id,
            name: name.to_string(),
            confidence_score: confidence,
            matched_identifiers: vec!["test".to_string()],
            platform: Some(platform.to_string()),
            platform_app_id: Some("12345".to_string()),
        }
    }

    #[test]
    fn test_single_candidate_high_confidence() {
        let scorer = ConfidenceScorer::new();
        let candidate = create_test_candidate(1, "Steam Game", "steam", 90.0);

        let confidence = scorer.calculate_single_candidate_confidence(&candidate);
        assert!(confidence.total_score > 95.0); // Platform boost should increase score
        assert!(confidence.is_reliable);
        assert_eq!(confidence.high_confidence_evidence, 1);
    }

    #[test]
    fn test_multiple_candidates_clear_winner() {
        let scorer = ConfidenceScorer::new();
        let candidates = vec![
            create_test_candidate(1, "Steam Game", "steam", 90.0),
            create_test_candidate(2, "Other Game", "standalone", 40.0),
        ];

        let (score, needs_confirmation, conflict) = scorer.calculate_overall_confidence(&candidates);
        assert!(score > 95.0);
        assert!(!needs_confirmation);
        assert!(conflict.is_none());
    }

    #[test]
    fn test_multiple_candidates_close_competition() {
        let scorer = ConfidenceScorer::new();
        let candidates = vec![
            create_test_candidate(1, "Steam Game", "steam", 75.0),
            create_test_candidate(2, "Epic Game", "epic", 70.0),
        ];

        let (score, needs_confirmation, conflict) = scorer.calculate_overall_confidence(&candidates);
        // Should still select the higher scoring one but recommend confirmation
        assert!(score > 80.0); // Steam boost
        assert!(needs_confirmation); // Close competition
        assert!(conflict.is_some());
    }

    #[test]
    fn test_validation_red_flags() {
        let scorer = ConfidenceScorer::new();
        let candidates = vec![
            create_test_candidate(1, "Unknown Game", "standalone", 20.0),
        ];

        let result = scorer.validate_identification(&candidates, 1).unwrap();
        assert!(result.should_warn);
        assert!(result.should_confirm);
        assert!(!result.red_flags.is_empty());
    }

    #[test]
    fn test_confidence_explanation() {
        let scorer = ConfidenceScorer::new();
        let candidate = create_test_candidate(1, "Test Game", "steam", 85.0);

        let explanation = scorer.explain_confidence_score(&vec![candidate], Some(1));
        assert!(explanation.contains("Test Game"));
        assert!(explanation.contains("confidence"));
        assert!(explanation.contains("evidence"));
    }
}

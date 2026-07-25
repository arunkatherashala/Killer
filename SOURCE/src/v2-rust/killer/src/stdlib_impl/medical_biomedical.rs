// ================================================================
// MEDICAL & BIOMEDICAL SOLVER - Phase 21.4
// Pharmacokinetics, epidemiology, diagnostics, clinical metrics
// Ported from: solver_medical_biomedical.killer
// ================================================================

use std::f64;

pub type Vector = Vec<f64>;

/// Medical and Biomedical Solver
pub struct MedicalSolver;

impl MedicalSolver {
    // ================================================================
    // PHARMACOKINETICS (1-20)
    // ================================================================

    /// Problem 1: One-Compartment Model - Exponential decay
    pub fn one_compartment_iv(dose: f64, vd: f64, ke: f64, t: f64) -> f64 {
        // C(t) = (dose/Vd) · exp(-ke·t)
        (dose / vd) * (-ke * t).exp()
    }

    /// Problem 2: Two-Compartment Model
    pub fn two_compartment_iv(dose: f64, vd: f64, k12: f64, k21: f64, ke: f64, t: f64) -> f64 {
        let discriminant = (ke + k12 + k21).powi(2) - 4.0 * ke * k21;
        if discriminant < 0.0 { return 0.0; }
        
        let sqrt_disc = discriminant.sqrt();
        let lambda1 = (ke + k12 + k21 + sqrt_disc) / 2.0;
        let lambda2 = (ke + k12 + k21 - sqrt_disc) / 2.0;
        
        let a = ((lambda1 - k21) * dose) / (vd * (lambda1 - lambda2));
        let b = ((k21 - lambda2) * dose) / (vd * (lambda1 - lambda2));
        
        a * (-lambda1 * t).exp() + b * (-lambda2 * t).exp()
    }

    /// Problem 3: Half-life Calculation
    pub fn half_life(ke: f64) -> f64 {
        0.693 / ke
    }

    /// Problem 4: Oral Bioavailability
    pub fn oral_bioavailability(cmax_oral: f64, cmax_iv: f64, dose_oral: f64, dose_iv: f64) -> f64 {
        (cmax_oral / cmax_iv) * (dose_iv / dose_oral)
    }

    /// Problem 5: Renal Clearance
    pub fn renal_clearance(urine_conc: f64, urine_flow: f64, plasma_conc: f64) -> f64 {
        if plasma_conc.abs() < 1e-14 { return 0.0; }
        (urine_conc * urine_flow) / plasma_conc
    }

    /// Problem 6: Total Clearance
    pub fn total_clearance(vd: f64, ke: f64) -> f64 {
        vd * ke
    }

    /// Problem 7: Steady-State Concentration
    pub fn steady_state_conc(dose: f64, clearance: f64, dosing_interval: f64) -> f64 {
        (dose / clearance) * (dosing_interval / (dosing_interval - dosing_interval))
    }

    /// Problem 8: Loading Dose
    pub fn loading_dose(target_conc: f64, vd: f64) -> f64 {
        target_conc * vd
    }

    /// Problem 9: Maintenance Dose
    pub fn maintenance_dose(target_conc: f64, clearance: f64, interval: f64) -> f64 {
        target_conc * clearance * interval
    }

    /// Problem 10: Creatinine Clearance (Cockcroft-Gault)
    pub fn creatinine_clearance(age: f64, weight: f64, serum_creatinine: f64, is_male: bool) -> f64 {
        let factor = if is_male { 140.0 } else { 130.0 };
        ((factor - age) * weight) / (72.0 * serum_creatinine)
    }

    // ================================================================
    // EPIDEMIOLOGY (11-25)
    // ================================================================

    /// Problem 11: SIR Model - Susceptible, Infected, Recovered
    pub fn sir_model_step(s: f64, i: f64, r: f64, beta: f64, gamma: f64, dt: f64, n: f64) -> (f64, f64, f64) {
        let ds = -beta * s * i / n;
        let di = beta * s * i / n - gamma * i;
        let dr = gamma * i;
        
        (
            (s + ds * dt).max(0.0),
            (i + di * dt).max(0.0),
            (r + dr * dt).max(0.0),
        )
    }

    /// Problem 12: Basic Reproduction Number (R₀)
    pub fn reproduction_number(beta: f64, gamma: f64) -> f64 {
        beta / gamma
    }

    /// Problem 13: Attack Rate
    pub fn attack_rate(cases: u32, exposed: u32) -> f64 {
        cases as f64 / exposed as f64
    }

    /// Problem 14: Case Fatality Rate
    pub fn case_fatality_rate(deaths: u32, cases: u32) -> f64 {
        deaths as f64 / cases as f64
    }

    /// Problem 15: Incidence Rate
    pub fn incidence_rate(new_cases: u32, population: u32, time_period_years: f64) -> f64 {
        (new_cases as f64 / population as f64) * (1.0 / time_period_years)
    }

    // ================================================================
    // DIAGNOSTIC TESTS (16-35)
    // ================================================================

    /// Problem 16: Sensitivity (True Positive Rate)
    pub fn sensitivity(tp: u32, fn_count: u32) -> f64 {
        tp as f64 / (tp + fn_count) as f64
    }

    /// Problem 17: Specificity (True Negative Rate)
    pub fn specificity(tn: u32, fp: u32) -> f64 {
        tn as f64 / (tn + fp) as f64
    }

    /// Problem 18: Positive Predictive Value (Precision)
    pub fn ppv(tp: u32, fp: u32) -> f64 {
        tp as f64 / (tp + fp) as f64
    }

    /// Problem 19: Negative Predictive Value
    pub fn npv(tn: u32, fn_count: u32) -> f64 {
        tn as f64 / (tn + fn_count) as f64
    }

    /// Problem 20: Likelihood Ratio Positive
    pub fn lr_positive(sens: f64, spec: f64) -> f64 {
        if (1.0 - spec).abs() < 1e-14 { return f64::INFINITY; }
        sens / (1.0 - spec)
    }

    /// Problem 21: Likelihood Ratio Negative
    pub fn lr_negative(sens: f64, spec: f64) -> f64 {
        if spec.abs() < 1e-14 { return f64::INFINITY; }
        (1.0 - sens) / spec
    }

    /// Problem 22: Accuracy
    pub fn accuracy(tp: u32, tn: u32, total: u32) -> f64 {
        (tp + tn) as f64 / total as f64
    }

    /// Problem 23: F1 Score (Harmonic mean of precision and recall)
    pub fn f1_score(tp: u32, fp: u32, fn_count: u32) -> f64 {
        let precision = tp as f64 / (tp + fp) as f64;
        let recall = tp as f64 / (tp + fn_count) as f64;
        
        if (precision + recall).abs() < 1e-14 { return 0.0; }
        2.0 * (precision * recall) / (precision + recall)
    }

    /// Problem 24: Youden's J Statistic
    pub fn youdens_j(sens: f64, spec: f64) -> f64 {
        sens + spec - 1.0
    }

    /// Problem 25: ROC AUC (Area Under Curve) - Simplified
    pub fn roc_auc(tp: u32, tn: u32, fp: u32, fn_count: u32) -> f64 {
        let sens = Self::sensitivity(tp, fn_count);
        let spec = Self::specificity(tn, fp);
        0.5 * (sens + spec)
    }

    // ================================================================
    // CLINICAL CALCULATIONS (26-45)
    // ================================================================

    /// Problem 26: BMI (Body Mass Index)
    pub fn bmi(weight_kg: f64, height_m: f64) -> f64 {
        weight_kg / (height_m * height_m)
    }

    /// Problem 27: BSA (Body Surface Area) - DuBois formula
    pub fn bsa_dubois(weight_kg: f64, height_cm: f64) -> f64 {
        0.007184 * weight_kg.powf(0.425) * height_cm.powf(0.725)
    }

    /// Problem 28: APACHE II Score (component)
    pub fn apache_ii_age_score(age: f64) -> u32 {
        match age as u32 {
            0..=44 => 0,
            45..=54 => 2,
            55..=64 => 3,
            65..=74 => 5,
            _ => 6,
        }
    }

    /// Problem 29: Glasgow Coma Scale
    pub fn gcs_score(eye_open: u32, verbal: u32, motor: u32) -> u32 {
        eye_open + verbal + motor
    }

    /// Problem 30: SOFA Score (Sepsis-related Organ Failure)
    pub fn sofa_respiratory(pao2_fio2: f64) -> u32 {
        match pao2_fio2 as u32 {
            400.. => 0,
            300..=399 => 1,
            200..=299 => 2,
            100..=199 => 3,
            _ => 4,
        }
    }

    // ================================================================
    // GENETICS & MOLECULAR (31-50)
    // ================================================================

    /// Problem 31: Hardy-Weinberg Equilibrium Check
    pub fn hardy_weinberg_p_allele(num_aa: usize, num_aa_heterozygote: usize, num_dominant: usize) -> f64 {
        let total_alleles = 2 * (num_aa + num_aa_heterozygote + num_dominant);
        let recessive_alleles = 2 * num_aa + num_aa_heterozygote;
        recessive_alleles as f64 / total_alleles as f64
    }

    /// Problem 32: Allele Frequency
    pub fn allele_frequency(recessive_count: usize, total_count: usize) -> f64 {
        recessive_count as f64 / (2 * total_count) as f64
    }

    /// Problem 33: GenDer Penetrance
    pub fn penetrance(affected_with_genotype: usize, total_with_genotype: usize) -> f64 {
        affected_with_genotype as f64 / total_with_genotype as f64
    }

    /// Problem 34: Relative Risk
    pub fn relative_risk(risk_exposed: f64, risk_unexposed: f64) -> f64 {
        if risk_unexposed.abs() < 1e-14 { return 0.0; }
        risk_exposed / risk_unexposed
    }

    /// Problem 35: Odds Ratio
    pub fn odds_ratio(tp: u32, fp: u32, fn_count: u32, tn: u32) -> f64 {
        let odds_disease = tp as f64 * tn as f64;
        let odds_no_disease = fp as f64 * fn_count as f64;
        if odds_no_disease.abs() < 1e-14 { return 0.0; }
        odds_disease / odds_no_disease
    }

    // ================================================================
    // LABORATORY CALCULATIONS (36-50)
    // ================================================================

    /// Problem 36: Anion Gap
    pub fn anion_gap(na: f64, cl: f64, hco3: f64) -> f64 {
        na - (cl + hco3)
    }

    /// Problem 37: Osmolal Gap
    pub fn osmolal_gap(serum_osmol: f64, calc_osmol: f64) -> f64 {
        serum_osmol - calc_osmol
    }

    /// Problem 38: GFR (Glomerular Filtration Rate) - MDRD
    pub fn gfr_mdrd(scr: f64, age: f64, is_male: bool) -> f64 {
        let factor = if is_male { 1.0 } else { 0.742 };
        186.0 * scr.powf(-1.154) * age.powf(-0.203) * factor
    }

    /// Problem 39: eGFR - CKD-EPI
    pub fn egfr_ckd_epi(scr: f64, age: f64, is_male: bool) -> f64 {
        let factor = if is_male { 1.0 } else { 1.018 };
        let exp = if is_male { -0.379 } else { -0.329 };
        141.0 * (scr / 0.7).min(1.0).powf(exp) * (scr / 0.7).max(1.0).powf(-1.209)
            * (0.993_f64).powf(age) * factor
    }

    /// Problem 40: Corrected Calcium
    pub fn corrected_calcium(serum_calcium: f64, albumin: f64) -> f64 {
        serum_calcium + 0.8 * (4.0 - albumin)
    }

    // ================================================================
    // PHARMACODYNAMICS (41-55)
    // ================================================================

    /// Problem 41: Emax Model
    pub fn emax_model(conc: f64, emax: f64, ec50: f64, gamma: f64) -> f64 {
        (emax * conc.powf(gamma)) / (ec50.powf(gamma) + conc.powf(gamma))
    }

    /// Problem 42: Hill Equation
    pub fn hill_equation(conc: f64, vmax: f64, km: f64, n: f64) -> f64 {
        (vmax * conc.powf(n)) / (km.powf(n) + conc.powf(n))
    }

    /// Problem 43: Michaelis-Menten Kinetics
    pub fn michaelis_menten(conc: f64, vmax: f64, km: f64) -> f64 {
        (vmax * conc) / (km + conc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensitivity_specificity() {
        let tp = 90u32;
        let fn_count = 10u32;
        let tn = 80u32;
        let fp = 20u32;
        
        let sens = MedicalSolver::sensitivity(tp, fn_count);
        let spec = MedicalSolver::specificity(tn, fp);
        
        assert!((sens - 0.9).abs() < 1e-10);
        assert!((spec - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_bmi() {
        let bmi = MedicalSolver::bmi(70.0, 1.75);
        assert!((bmi - 22.857).abs() < 0.01);
    }

    #[test]
    fn test_one_compartment() {
        let c = MedicalSolver::one_compartment_iv(1000.0, 50.0, 0.1, 0.0);
        assert!((c - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_odds_ratio() {
        let or = MedicalSolver::odds_ratio(50, 20, 30, 100);
        assert!((or - 8.33).abs() < 0.1);
    }
}

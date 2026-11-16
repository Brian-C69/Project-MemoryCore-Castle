//! The same indicator math `MarketChart.vue` computes for display (`indicators.js`), ported here
//! so the companion's own market context can include real TA numbers instead of just raw
//! candles. Kept in lock-step with the JS version - same formulas, same test vectors - so the
//! chart the member sees and the numbers the companion reasons from never quietly diverge.

/// The simple moving average of `values` over `period` bars; `None` before the window fills.
pub fn sma(values: &[f64], period: usize) -> Vec<Option<f64>> {
    let mut out = vec![None; values.len()];
    if period == 0 {
        return out;
    }
    let mut sum = 0.0;
    for i in 0..values.len() {
        sum += values[i];
        if i >= period {
            sum -= values[i - period];
        }
        if i + 1 >= period {
            out[i] = Some(sum / period as f64);
        }
    }
    out
}

/// The latest (last) value of `sma`, for a compact summary rather than the whole series.
pub fn latest_sma(values: &[f64], period: usize) -> Option<f64> {
    sma(values, period).last().copied().flatten()
}

/// Wilder's RSI of `closes` over `period` (14 is the usual default); `None` before it starts.
pub fn rsi(closes: &[f64], period: usize) -> Vec<Option<f64>> {
    let mut out = vec![None; closes.len()];
    if period == 0 || closes.len() <= period {
        return out;
    }
    let mut gain_sum = 0.0;
    let mut loss_sum = 0.0;
    for i in 1..=period {
        let diff = closes[i] - closes[i - 1];
        if diff > 0.0 {
            gain_sum += diff;
        } else {
            loss_sum -= diff;
        }
    }
    let mut avg_gain = gain_sum / period as f64;
    let mut avg_loss = loss_sum / period as f64;
    out[period] = Some(rsi_of(avg_gain, avg_loss));
    for i in (period + 1)..closes.len() {
        let diff = closes[i] - closes[i - 1];
        let gain = diff.max(0.0);
        let loss = (-diff).max(0.0);
        avg_gain = (avg_gain * (period - 1) as f64 + gain) / period as f64;
        avg_loss = (avg_loss * (period - 1) as f64 + loss) / period as f64;
        out[i] = Some(rsi_of(avg_gain, avg_loss));
    }
    out
}

/// The latest RSI reading, for a compact summary rather than the whole series.
pub fn latest_rsi(closes: &[f64], period: usize) -> Option<f64> {
    rsi(closes, period).last().copied().flatten()
}

fn rsi_of(avg_gain: f64, avg_loss: f64) -> f64 {
    if avg_loss == 0.0 {
        return if avg_gain == 0.0 { 50.0 } else { 100.0 };
    }
    100.0 - 100.0 / (1.0 + avg_gain / avg_loss)
}

/// %K and %D of the stochastic oscillator; both `None` before they have enough bars.
pub fn stochastic(highs: &[f64], lows: &[f64], closes: &[f64], k_period: usize, d_period: usize) -> (Vec<Option<f64>>, Vec<Option<f64>>) {
    let n = closes.len();
    let mut k = vec![None; n];
    if k_period > 0 {
        for i in 0..n {
            if i + 1 < k_period {
                continue;
            }
            let window = (i + 1 - k_period)..=i;
            let highest = window.clone().map(|j| highs[j]).fold(f64::MIN, f64::max);
            let lowest = window.map(|j| lows[j]).fold(f64::MAX, f64::min);
            let range = highest - lowest;
            k[i] = Some(if range == 0.0 { 50.0 } else { 100.0 * (closes[i] - lowest) / range });
        }
    }
    let d = sma_over_options(&k, d_period);
    (k, d)
}

/// The latest %K and %D, for a compact summary rather than the whole series.
pub fn latest_stochastic(highs: &[f64], lows: &[f64], closes: &[f64], k_period: usize, d_period: usize) -> (Option<f64>, Option<f64>) {
    let (k, d) = stochastic(highs, lows, closes, k_period, d_period);
    (k.last().copied().flatten(), d.last().copied().flatten())
}

/// `sma`, but a `None` resets the averaging window instead of poisoning it.
fn sma_over_options(values: &[Option<f64>], period: usize) -> Vec<Option<f64>> {
    let mut out = vec![None; values.len()];
    if period == 0 {
        return out;
    }
    let mut window: Vec<f64> = Vec::new();
    let mut sum = 0.0;
    for (i, value) in values.iter().enumerate() {
        match value {
            None => {
                window.clear();
                sum = 0.0;
            }
            Some(v) => {
                window.push(*v);
                sum += v;
                if window.len() > period {
                    sum -= window.remove(0);
                }
                if window.len() == period {
                    out[i] = Some(sum / period as f64);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close_to(a: Option<f64>, b: f64) -> bool {
        a.is_some_and(|v| (v - b).abs() < 1e-6)
    }

    #[test]
    fn sma_averages_the_trailing_window_once_it_has_enough_bars() {
        let out = sma(&[1.0, 2.0, 3.0, 4.0, 5.0], 3);
        assert_eq!(out[0], None);
        assert_eq!(out[1], None);
        assert!(close_to(out[2], 2.0));
        assert!(close_to(out[3], 3.0));
        assert!(close_to(out[4], 4.0));
    }

    #[test]
    fn sma_is_none_everywhere_when_the_period_does_not_fit() {
        assert_eq!(sma(&[1.0, 2.0], 5), vec![None, None]);
    }

    #[test]
    fn rsi_reaches_100_or_0_at_the_extremes_and_50_when_flat() {
        let up: Vec<f64> = (10..25).map(|n| n as f64).collect();
        assert!(close_to(latest_rsi(&up, 14), 100.0));
        let down: Vec<f64> = up.iter().rev().copied().collect();
        assert!(close_to(latest_rsi(&down, 14), 0.0));
        let flat = vec![10.0; 16];
        assert!(close_to(latest_rsi(&flat, 14), 50.0));
    }

    #[test]
    fn rsi_is_none_until_the_period_has_filled() {
        let closes = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(rsi(&closes, 14), vec![None; closes.len()]);
    }

    #[test]
    fn stochastic_k_places_the_close_within_the_recent_high_low_range() {
        let highs = [100.0, 105.0, 110.0, 108.0, 107.0];
        let lows = [90.0, 92.0, 90.0, 95.0, 96.0];
        let closes = [95.0, 100.0, 100.0, 101.0, 96.0];
        // over the 5-bar window the range is 90..110; the last close (96) sits 30% up it
        let (k, _) = stochastic(&highs, &lows, &closes, 5, 3);
        assert!(close_to(k[4], 30.0));
        assert_eq!(&k[0..4], &[None, None, None, None]);
    }

    #[test]
    fn stochastic_d_is_ks_own_moving_average() {
        let highs = [10.0; 6];
        let lows = [0.0; 6];
        let closes = [5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let (k, d) = stochastic(&highs, &lows, &closes, 3, 2);
        assert_eq!(d[0], None);
        assert_eq!(d[2], None); // %K just started at index 2, needs one more for %D
        assert!(close_to(d[3], (k[2].unwrap() + k[3].unwrap()) / 2.0));
    }

    #[test]
    fn stochastic_treats_a_flat_range_as_the_middle_never_dividing_by_zero() {
        let (k, _) = stochastic(&[10.0, 10.0], &[10.0, 10.0], &[10.0, 10.0], 2, 1);
        assert!(close_to(k[1], 50.0));
    }
}

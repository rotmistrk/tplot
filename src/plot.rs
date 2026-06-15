//! Text-based plotting — bar charts and simple line plots using Unicode.
//! No external dependencies. Gnuplot used when available for high-quality output.

/// A single data series for plotting.
pub(crate) struct Series {
    pub(crate) label: String,
    pub(crate) values: Vec<(String, f64)>, // (x_label, y_value)
}

/// Render a horizontal bar chart as text lines.
pub(crate) fn bar_chart(series: &[Series], width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let all_values: Vec<f64> = series.iter().flat_map(|s| s.values.iter().map(|(_, v)| *v)).collect();
    let max_val = all_values.iter().copied().fold(0.0f64, f64::max);
    if max_val == 0.0 {
        return vec!["(no data)".to_string()];
    }

    let label_width = series
        .iter()
        .flat_map(|s| s.values.iter().map(|(l, _)| l.len()))
        .max()
        .unwrap_or(5)
        .min(20);
    let bar_width = width.saturating_sub(label_width + 12); // label + space + bar + value

    for s in series {
        if series.len() > 1 {
            lines.push(format!("── {} ──", s.label));
        }
        for (label, val) in &s.values {
            let bar_len = ((val / max_val) * bar_width as f64) as usize;
            let bar: String = "█".repeat(bar_len);
            let lbl: String = label.chars().take(label_width).collect();
            lines.push(format!("{:<width$} │{} {:.1}", lbl, bar, val, width = label_width));
        }
    }
    lines
}

/// Render a simple line chart using braille characters.
pub(crate) fn line_chart(series: &[Series], width: usize, height: usize) -> Vec<String> {
    if series.is_empty() || height < 3 {
        return vec!["(no data)".to_string()];
    }

    let all_values: Vec<f64> = series.iter().flat_map(|s| s.values.iter().map(|(_, v)| *v)).collect();
    let min_val = all_values.iter().copied().fold(f64::INFINITY, f64::min);
    let max_val = all_values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let range = if (max_val - min_val).abs() < 1e-10 {
        1.0
    } else {
        max_val - min_val
    };

    let plot_height = height.saturating_sub(2); // leave room for axis labels
    let plot_width = width.saturating_sub(8); // leave room for y-axis

    // Build a character grid.
    let mut grid = vec![vec![' '; plot_width]; plot_height];

    let markers = ['●', '○', '◆', '◇', '■', '□'];
    for (si, s) in series.iter().enumerate() {
        let marker = markers[si % markers.len()];
        let n = s.values.len();
        if n == 0 {
            continue;
        }
        for (i, (_, val)) in s.values.iter().enumerate() {
            let x = if n > 1 {
                i * (plot_width - 1) / (n - 1)
            } else {
                plot_width / 2
            };
            let y_frac = (val - min_val) / range;
            let y = plot_height.saturating_sub(1) - ((y_frac * (plot_height - 1) as f64) as usize).min(plot_height - 1);
            if x < plot_width && y < plot_height {
                grid[y][x] = marker;
            }
        }
    }

    let mut lines = Vec::new();
    // Y-axis labels + grid.
    for (row_idx, row) in grid.iter().enumerate() {
        let y_val = max_val - (row_idx as f64 / (plot_height - 1) as f64) * range;
        let label = format!("{:>6.1}", y_val);
        let row_str: String = row.iter().collect();
        lines.push(format!("{} │{}", label, row_str));
    }
    // X-axis.
    lines.push(format!("{:>6} └{}", "", "─".repeat(plot_width)));

    // Legend.
    if series.len() > 1 {
        let legend: String = series
            .iter()
            .enumerate()
            .map(|(i, s)| format!("{} {}", markers[i % markers.len()], s.label))
            .collect::<Vec<_>>()
            .join("  ");
        lines.push(format!("       {legend}"));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bar_chart() {
        let s = Series {
            label: "users".into(),
            values: vec![("root".into(), 3.0), ("admin".into(), 1.0), ("deploy".into(), 1.0)],
        };
        let lines = bar_chart(&[s], 60);
        assert!(!lines.is_empty());
        assert!(lines[0].contains("root"));
        assert!(lines[0].contains("█"));
    }

    #[test]
    fn test_multi_series_bar() {
        let s1 = Series {
            label: "success".into(),
            values: vec![("10:00".into(), 5.0), ("10:01".into(), 3.0)],
        };
        let s2 = Series {
            label: "failure".into(),
            values: vec![("10:00".into(), 2.0), ("10:01".into(), 7.0)],
        };
        let lines = bar_chart(&[s1, s2], 60);
        assert!(lines.iter().any(|l| l.contains("success")));
        assert!(lines.iter().any(|l| l.contains("failure")));
    }

    #[test]
    fn test_line_chart() {
        let s = Series {
            label: "rate".into(),
            values: vec![
                ("1".into(), 1.0),
                ("2".into(), 4.0),
                ("3".into(), 2.0),
                ("4".into(), 8.0),
            ],
        };
        let lines = line_chart(&[s], 40, 10);
        assert!(lines.len() >= 8);
    }
}

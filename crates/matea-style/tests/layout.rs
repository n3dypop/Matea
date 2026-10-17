use matea_style::{Length, distribute};

#[test]
fn weighted_layout_preserves_budget_zero_weights_and_fixed_priority() {
    let lengths = [
        Length::Fixed(0),
        Length::Fixed(1),
        Length::Fixed(u16::MAX),
        Length::Fill(0),
        Length::Fill(1),
        Length::Fill(u16::MAX),
    ];
    for total in [0, 1, 2, 7, 1024, u16::MAX] {
        for a in lengths {
            for b in lengths {
                for c in lengths {
                    let input = [a, b, c];
                    let out = distribute(total, &input);
                    let used: u32 = out.iter().map(|&n| u32::from(n)).sum();
                    assert!(used <= u32::from(total));
                    let mut fixed_budget = total;
                    for (&l, &n) in input.iter().zip(&out) {
                        match l {
                            Length::Fixed(requested) => {
                                assert_eq!(n, requested.min(fixed_budget));
                                fixed_budget -= n;
                            }
                            Length::Fill(0) => assert_eq!(n, 0),
                            _ => {}
                        }
                    }
                    if input.iter().any(|l| matches!(l, Length::Fill(1..))) {
                        assert_eq!(used, u32::from(total));
                    }
                }
            }
        }
    }
}

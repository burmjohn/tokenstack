# Evidence 05: Analytics And Timezone

Generated: 2026-07-02

## Implemented

- Daily, month-to-date, lifetime, peak, heatmap, reset timeline, and source coverage DTOs exist in `src-tauri/src/analytics.rs`.
- Reset-credit expiration formatting explicitly uses `America/New_York`.
- DST spring-forward and fall-back tests cover timezone edge cases.

## Fresh Verification

`cargo test --manifest-path src-tauri/Cargo.toml --no-default-features` passed:

- `computes_lifetime_tokens`
- `computes_today_in_america_new_york`
- `computes_month_to_date_in_america_new_york`
- `formats_reset_expiration_in_america_new_york`
- `handles_dst_spring_forward`
- `handles_dst_fall_back`
- `handles_zero_data_without_nan_or_crash`

# Porting Python & Data Science (`NumPy` / `SciPy` / `Pandas` / `Scikit-Learn`) to Pure Axon

## 1. Idiom Translation Cheatsheet

| Python / Data Science | Pure Axon (`v1.2.0`) |
|---|---|
| `df = pd.read_csv("sales.csv")` | `df := df_read_csv("sales.csv") or return -1` (`use std::dataframe`) |
| `df[df["price"] > 50.0]` | `sub := df_filter_f64(df, "price", ">", 50.0)` |
| `df.groupby("region")["rev"].mean()` | `agg := df_groupby_agg(df, "region", "rev", "mean")` |
| `pd.merge(a, b, on="id", how="left")` | `j := df_join(a, b, "id", "id", "left")` |
| `X = df[["f1","f2"]].to_numpy()` | `x := df_to_mat_f64(df, "f1,f2")` (`use std::linalg`) |
| `np.linalg.solve(A, b)` / `lstsq` | `x := lu_solve(a, b) or return -1` / `lstsq_solve(a, b)` |
| `u, s, vt = np.linalg.svd(A)` | `sv := svd_jacobi(a) or return -1` |
| `scipy.fft.fft(x)` | `fft_radix2_inplace(re, im, false)` (`use std::numeric`) |
| `scipy.optimize.brentq(f, a, b)` | `mut s := brent_init(a, f(a), b, f(b)) or return -1; while not s.converged { _ := brent_step(mut s, f(s.x_eval)) }` |
| `scipy.stats.ttest_ind(a, b, equal_var=False)` | `res := welch_ttest(a, b)` (`res.stat`, `res.p_value` via `use std::stats`) |
| `sm.OLS(y, X).fit()` | `ols := ols_fit_summary(x_design, y) or return -1` (`ols.r2`, `ols.p_values`, `ols.aic`) |
| `RandomForestClassifier(n_estimators=20)` | `rf := ml_rf_fit(x_tr, y_tr, 20, 5, true, 42)` (`use std::ml`) |
| `GradientBoostingRegressor()` | `gb := ml_gbdt_fit(x_tr, y_tr, 30, 0.1, 3, false)` |
| `roc_auc_score(y_true, y_score)` | `auc := ml_roc_auc_score(y_true, y_score)` |

## 2. Performance & Ownership Rules When Porting Python
1. **Eliminate Row-by-Row Dict Loops:** Keep tabular data in `DataFrame` (`std::dataframe`) and numeric tensors in aligned `MatF64` (`std::linalg`) so `AxIR` emits 8-wide `AVX2+FMA3` (`VFMADD231PD`) machine code (`50x–200x` faster than CPython loops).
2. **Deterministic Cleanup (`0` GC Pauses):** Call `df_free(move df)`, `mat_free(move m)`, `ml_rf_free(move rf)` when finished with large matrices/tables.
3. **Float Literals:** Write `0.000001` instead of `1e-6`, and use `and`/`or`/`not` (same words as Python!).

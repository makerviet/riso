# RASO — Bộ máy tính điểm kiểu FIRST (`raso-rules`) — Thiết kế

- **Ngày:** 15/10/2026
- **Trạng thái:** chờ duyệt
- **Giai đoạn:** 1 trong `docs/ROADMAP.md`

## 1. Mục tiêu

Một bộ máy tính điểm duy nhất, khai báo bằng file, dùng chung cho:

1. **Trận mô phỏng:** server RASO tự đo trạng thái sân và tính điểm.
2. **Trận thật:** trọng tài nhập biên bản trên máy tính bảng tại giải (FAnRoC, buổi tập, giải FGC thử).

Giáo viên và học sinh khá phải tự viết được luật cho một đề mới mà không cần sửa code Rust.

### Tiêu chí thành công

- FGC 2026 *Igniting Innovation* và FGC 2025 *Eco Equilibrium* có file luật chạy được, khớp game manual; bộ test của mỗi đề xanh.
- Trong mô phỏng FGC 2026, mọi điểm đều đi qua `raso-rules`; không còn điểm viết cứng trong Rust.
- Trang `/referee/<đề>` chấm một trận thật hoàn toàn trong trình duyệt bằng WASM, kết quả trùng từng chữ số với bản Rust.
- Cùng một biên bản thì mô phỏng, trang trọng tài và CLI cho cùng kết quả.

### Ngoài phạm vi

- Xếp lịch thi đấu, playoff, chọn liên minh, lưu kết quả giải lên server: giai đoạn 2.
- Chế độ lớp học, tự host offline, PWA: giai đoạn 3.
- Đề FAnRoC (SeaHeal 2026–2027 đang có 6 chỗ mâu thuẫn chờ BTC trả lời; xem phụ lục C) và các đề FGC cũ hơn: giai đoạn 5.
- Mô phỏng robot nâng robot (Partner climb) và luồng Human Player nạp Extinguisher.

## 2. Bối cảnh: hiện trạng tính điểm

| Luật FGC 2026 | Hiện trạng | Vị trí |
|---|---|---|
| Bóng trong Suppression Unit | Chạy, viết cứng | `server/src/game/sphere_runtime/scoring.rs` |
| Hệ số leo | Chạy, viết cứng, áp lúc kết thúc | `sphere_runtime.rs` `brace_multipliers`, `apply_su_multiplier`; `match_registry.rs` nhánh `brace_score_applied` |
| Partner climb, Coopertition, Knockdown, lỗi phạt | Không chạy | `rules/scoring.rhai` `on_match_end`, `rules/penalties.rhai` không bao giờ được gọi |
| Extinguisher | Tắt | `manifest.json` `enabled: false` |

Chỉ hook `on_trigger_enter` được nối (`rhai_engine.rs:99`).

## 3. Kiến trúc

```
pkgs/games/<id>/rules.yaml      luật (công khai)          ┐
pkgs/games/<id>/observe.yaml    ánh xạ đo mô phỏng (riêng) │ gói đề
pkgs/games/<id>/tests/scoring/  biên bản + kết quả mẫu    ┘
                │
        crates/raso-rules  (Rust, thuần, không vật lý)
          ├─ lib:  validate / score / rank
          ├─ bin:  raso (CLI)
          └─ wasm: wasm-bindgen → crates/raso-rules/pkg
                │                         │
        server (path dep)            web /referee/<id>
        đo → biên bản → score        nhập tay → biên bản → score
```

### 3.1 API của thư viện

Mọi hàm đều thuần (cùng đầu vào thì cùng đầu ra), không panic, lỗi trả về dạng giá trị.

```rust
pub fn load(yaml: &str, resolver: &dyn PackResolver) -> Result<Rules, Vec<RuleError>>;
pub fn score(rules: &Rules, obs: &Observation) -> Result<ScoreSheet, Vec<ScoreError>>;
pub fn rank(rules: &Rules, results: &[MatchResult]) -> Result<Standings, Vec<ScoreError>>;
```

- `load` gồm cả `validate`. `PackResolver` cung cấp YAML của đề cha khi dùng `extends`.
- `Observation`: biên bản cuối trận (mục 5).
- `ScoreSheet`: từng giá trị có `label` theo cấp field/side/robot, điểm phạt, điểm cuối mỗi liên minh, điểm từng đội sau khi áp thẻ, các mục `not_simulated`.
- `MatchResult`: gồm `Observation` và `ScoreSheet` của một trận, kèm danh sách đội theo liên minh.

## 4. Định dạng `rules.yaml` (schema `raso.rules/v1`)

### 4.1 Khối cấp cao

| Khối | Bắt buộc | Ý nghĩa |
|---|---|---|
| `schema` | có | `raso.rules/v1` |
| `game`, `name`, `version`, `source` | có | id đề, tên, phiên bản luật (semver, tăng khi đổi điểm), trích dẫn game manual |
| `extends` + `override` | không | kế thừa đề khác (mục 4.6) |
| `sides` | có | `ids` (ví dụ `[red, blue]`), `teams` mỗi liên minh |
| `rounding` | có | `ceil` \| `floor` \| `round` \| `none`, áp cho điểm cuối mỗi liên minh |
| `inputs` | có | ô biên bản theo cấp `field` / `side` / `robot` |
| `constants` | không | hằng số và bảng tra |
| `values` | có | giá trị tính, theo cấp `robot` / `side` / `field` |
| `fouls` | không | lỗi cấp liên minh |
| `cards` | không | thẻ cấp đội |
| `score` | có | công thức điểm cuối một liên minh (trước làm tròn) |
| `ranking` | có | cách xếp hạng |
| `open_questions` | không | chỗ luật chưa rõ, cách hiểu đang dùng, nguồn |

### 4.2 Ô biên bản (`inputs`)

| `type` | Thuộc tính | Giao diện trọng tài |
|---|---|---|
| `count` | `max` (tuỳ chọn) | nút −/+ |
| `choice` | `values: [...]` | hàng nút |
| `bool` | | công tắc |

Mỗi ô có `label` tiếng Việt; tuỳ chọn có `label_en` và `help`.

### 4.3 Công thức

- Công thức là **biểu thức Rhai** (feature `decimal` + `sync`, **không** dùng `no_float` vì Cargo gộp feature với Rhai của server và sẽ tắt số thực trong `arena.rhai`/`robot.rhai`). Trước khi biên dịch, mọi chữ số ngoài chuỗi được đổi thành `__d("…")` → `rust_decimal::Decimal`; mọi ô biên bản cũng nạp dạng `Decimal`. Nhờ vậy `0.05` là số thập phân chính xác và `1 / 3` = 0,333… (không phải phép chia nguyên).
- **Tên dùng được trong công thức:**

| Cấp đang tính | Tên truy cập được |
|---|---|
| `robot` | các ô `robot` của chính robot đó; `constants` |
| `side` | ô và giá trị `side` của chính liên minh; `robots.<tên>` (mảng giá trị robot của liên minh); `field.<tên>` (ô và giá trị field); `<side_id>.<tên ô side>` và `opponent.<tên ô side>` (**chỉ ô biên bản**, không phải giá trị tính, để tránh phụ thuộc chéo giữa hai liên minh); `constants` |
| `field` | ô `field`; `all_robots.<tên>` (mảng giá trị robot toàn sân); `<side_id>.<tên ô side>`; `constants` |

- `bool` được tính là 1/0 khi đưa vào `sum` và `count`.
- **Hàm có sẵn:** `sum(arr)`, `count(arr)` (đếm phần tử đúng/khác 0), `len(arr)` (số phần tử), `min`, `max`, `avg`, `stddev(arr)` (độ lệch chuẩn tổng thể), `ceil`, `floor`, `round`, `abs`, `lookup(x, bảng)` (khớp đúng khoá, không có khoá → lỗi `EVAL`), `steps(n, bảng)` (giá trị của khoá số lớn nhất ≤ n, không khoá nào thì 0).
- `if a { x } else if b { y } else { z }` được dùng như một biểu thức; đây là cách viết bảng tra theo khoảng.
- **Thứ tự tính:** sắp xếp topo theo quan hệ phụ thuộc giữa các tên. Vòng lặp phụ thuộc → lỗi `validate`.
- **Giá trị có `label`** xuất hiện trong bảng điểm chi tiết. Viết tắt `tên: "biểu thức"` tương đương `{ expr: ..., label: null }`.

### 4.4 Lỗi phạt (`fouls`) và thẻ (`cards`)

```yaml
fouls:
  minor: { label: "...", points: "<biểu thức cấp side>", goes_to: opponent | self }
```

- Trọng tài nhập **số lần** mỗi loại lỗi cho mỗi liên minh. Mô phỏng ở giai đoạn 1 luôn nhập 0.
- Công thức `points` được tính ở cấp side của **liên minh phạm lỗi**, rồi nhân với số lần.
- `goes_to: opponent` cộng vào biến `fouls_received` của đối thủ; `goes_to: self` cộng vào biến `fouls_committed` của chính liên minh đó (là số dương; công thức `score` tự quyết định trừ ở đâu).
- Công thức `points` **không được** tham chiếu `fouls_received`, `fouls_committed` hay `score`. Vi phạm thì `validate` báo lỗi.

`cards` khai báo hậu quả mỗi loại thẻ cấp đội: `team_zero` (đội đó nhận 0 điểm trận khi xếp hạng; điểm liên minh và các đội khác không đổi) hoặc `none`. Trọng tài chọn thẻ cho từng đội.

### 4.5 Xếp hạng (`ranking`)

```yaml
ranking:
  aggregate: average | sum
  drop_lowest: { count: 1, not_if: [red] }   # tuỳ chọn
  keep_best: 4                               # tuỳ chọn, loại trừ với drop_lowest
  tiebreakers:
    - { label: "...", expr: "<biểu thức trên matches>" }
  final_tiebreak: draw | team_number
```

- Giá trị trận của một đội = điểm cuối liên minh của đội đó, hoặc 0 nếu đội bị thẻ `team_zero`.
- `matches.<tên>` là mảng giá trị cấp side (của liên minh đội đó) hoặc `team_score`, **chỉ gồm các trận được tính** (sau khi bỏ trận thấp nhất hoặc chỉ giữ các trận tốt nhất).
- Trận bị thẻ trong danh sách `not_if` không được chọn làm trận bỏ đi; khi đó trận thấp nhất kế tiếp bị bỏ.
- `draw` để giai đoạn 2 bốc thăm; giai đoạn 1 trả trạng thái "đồng hạng".

### 4.6 Kế thừa

```yaml
extends: fgc-2026
override:
  constants.climb_increment.ZONE_3: 0.25
  values.side.partner: { expr: "20 * count(robots.lifted_by_partner)", label: "Partner climb" }
```

- Khoá `override` là đường dẫn chấm, có thể thay một lá hoặc cả một nhánh.
- Đề con kế thừa `inputs`, `constants`, `values`, `fouls`, `cards`, `score`, `ranking`. Bộ test của đề cha **không** tự kế thừa.
- Cấm kế thừa vòng. Độ sâu tối đa 4.

### 4.7 Hộp cát

- Công thức không được dùng `eval`, `import`, `fn`, `while`, `loop`, `for`, `print`, `debug`; các ký hiệu này bị tắt ở `Engine`.
- Mỗi công thức tối đa 10.000 phép toán, độ sâu biểu thức tối đa 64.
- Luật lỗi bị từ chối ở bước `load`, không bao giờ lọt đến lúc tính điểm.

## 5. Biên bản (`Observation`)

```yaml
game: fgc-2026
rules_version: "1.0.0"
field:  { ext_wildfire: 0 }
sides:
  red:
    su_wildfire: 120
    robots:  [ { climb: ZONE_2, lifted_by_partner: false }, {...}, {...} ]
    fouls:   { minor: 0, major: 0 }
    teams:   [ { id: "VIE" }, { id: "LAO" }, { id: "BRU", card: yellow } ]   # tuỳ chọn; thiếu thì không áp thẻ
  blue: {...}
source: referee | simulation
not_simulated: [ "robot.lifted_by_partner" ]   # chỉ khi source = simulation
```

Validate biên bản: đủ ô, `count` là số nguyên ≥ 0 và ≤ `max`, `choice` hợp lệ, số robot mỗi liên minh = `sides.teams`, `card` thuộc `cards`. Ô `count` là số nguyên (`i64`).

## 6. Hai đề mẫu

### 6.1 FGC 2026 — Igniting Innovation

Nguồn: Game Manual mục 3.6; đối chiếu máy tính điểm chính thức (first.global/calculator).

```yaml
schema: raso.rules/v1
game: fgc-2026
name: Igniting Innovation
version: 1.0.0
source: "FGC 2026 Game Manual §3.6, §6.3"
sides: { ids: [red, blue], teams: 3 }
rounding: ceil
inputs:
  field:
    ext_wildfire: { type: count, max: 500, label: "WILDFIRE trong Extinguisher" }
  side:
    su_wildfire:  { type: count, max: 500, label: "WILDFIRE trong Suppression Unit" }
  robot:
    climb: { type: choice, values: [NONE, CONTACT, ZONE_1, ZONE_2, ZONE_3], label: "Vị trí leo (kể cả được nâng gián tiếp)" }
    lifted_by_partner: { type: bool, label: "Được robot đồng đội nâng" }
constants:
  climb_increment: { NONE: 0, CONTACT: 0.05, ZONE_1: 0.10, ZONE_2: 0.20, ZONE_3: 0.30 }
  coop_bonus: { 4: 10, 5: 25, 6: 40 }
values:
  robot:
    climb_inc: "lookup(climb, climb_increment)"
    in_zone3:  "climb == \"ZONE_3\""
  field:
    ext_points: { expr: "ext_wildfire", label: "Extinguisher" }
    knockdown:  { expr: "if ext_wildfire > red.su_wildfire && ext_wildfire > blue.su_wildfire { 10 } else { 0 }", label: "Knockdown" }
    coop:       { expr: "steps(count(all_robots.in_zone3), coop_bonus)", label: "Coopertition" }
  side:
    climb_mult:  { expr: "1 + sum(robots.climb_inc)", label: "Hệ số leo" }
    su_points:   { expr: "ceil(su_wildfire * climb_mult)", label: "Suppression Unit × hệ số" }
    partner:     { expr: "25 * count(robots.lifted_by_partner)", label: "Partner climb" }
    pre_penalty: "su_points + partner + field.ext_points + field.knockdown + field.coop"
fouls:
  minor: { label: "Thẻ vàng + lỗi nhẹ (5%)",  points: "0.05 * pre_penalty", goes_to: opponent }
  major: { label: "Thẻ vàng + lỗi nặng (10%)", points: "0.10 * pre_penalty", goes_to: opponent }
cards: { white: team_zero, yellow: none, red: team_zero }
score: "pre_penalty + fouls_received"
ranking:
  aggregate: average
  drop_lowest: { count: 1, not_if: [red] }
  tiebreakers:
    - { label: "Trận cao nhất", expr: "max(matches.team_score)" }
    - { label: "Tổng điểm Suppression Unit", expr: "sum(matches.su_points)" }
  final_tiebreak: draw
open_questions:
  - id: knockdown
    used: "Extinguisher > từng Suppression Unit (bằng nhau thì không)"
    source: "Máy tính chính thức: wf_e > wf_r && wf_e > wf_b; manual §3.4 ghi 'both of the two'"
  - id: su_round_step
    used: "Làm tròn lên ngay ở su_points"
    source: "Máy tính chính thức: Math.ceil(suppWF*(1+multInc))"
  - id: tiebreak_2
    used: "Tổng su_points (đã nhân hệ số) của các trận được tính"
    source: "Manual §6.3 ghi 'SUPPRESSION UNIT points'; máy tính chính thức ghi head-to-head → theo manual"
```

Làm tròn lên ở bước `su_points` cộng với `rounding: ceil` ở điểm cuối cho ra cùng kết quả với máy tính chính thức, vì các khoản còn lại đều là số nguyên.

### 6.2 FGC 2025 — Eco Equilibrium

Nguồn: Game Manual mục 3.2–3.6, 6.3; máy tính điểm Google Sheets chính thức.

```yaml
schema: raso.rules/v1
game: fgc-2025
name: Eco Equilibrium
version: 1.0.0
source: "FGC 2025 Game Manual §3.6, §6.3"
sides: { ids: [red, blue], teams: 3 }
rounding: ceil
inputs:
  field:
    barriers_mitigated:     { type: count, max: 9,   label: "BARRIER trong MITIGATOR" }
    barriers_in_ecosystems: { type: count, max: 9,   label: "BARRIER còn trong ECOSYSTEM" }
    bio_freshwater:         { type: count, max: 120, label: "BIODIVERSITY UNIT — Freshwater" }
    bio_marine:             { type: count, max: 120, label: "BIODIVERSITY UNIT — Marine" }
    bio_terrestrial:        { type: count, max: 120, label: "BIODIVERSITY UNIT — Terrestrial" }
  robot:
    rope: { type: choice, values: [NONE, LEVEL_1, LEVEL_2, LEVEL_3, LEVEL_4], label: "Vị trí trên ROPE" }
constants:
  rope_increment: { NONE: 0, LEVEL_1: 0.125, LEVEL_2: 0.25, LEVEL_3: 0.375, LEVEL_4: 0.5 }
  coop_bonus: { 5: 15, 6: 30 }
values:
  robot:
    rope_inc:  "lookup(rope, rope_increment)"
    supported: "rope == \"LEVEL_2\" || rope == \"LEVEL_3\" || rope == \"LEVEL_4\""
  field:
    barrier_points: { expr: "barriers_mitigated", label: "BARRIER" }
    bio_units:      { expr: "bio_freshwater + bio_marine + bio_terrestrial", label: "BIODIVERSITY UNIT" }
    sigma:          { expr: "stddev([bio_freshwater, bio_marine, bio_terrestrial])", label: "Độ lệch chuẩn σ" }
    distribution:   { expr: "if barriers_in_ecosystems > 0 { 0.5 } else if sigma <= 1 { 1.0 } else if sigma < 10 { 0.6 } else { 0.5 }", label: "Distribution factor" }
    coop:           { expr: "steps(count(all_robots.supported), coop_bonus)", label: "Coopertition" }
  side:
    protection: { expr: "1 + sum(robots.rope_inc)", label: "Protection multiplier" }
    pre_penalty: "(field.barrier_points + field.bio_units * field.distribution) * protection + field.coop"
fouls:
  minor: { label: "Thẻ vàng + lỗi nhẹ (5%)",  points: "0.05 * pre_penalty", goes_to: opponent }
  major: { label: "Thẻ vàng + lỗi nặng (10%)", points: "0.10 * pre_penalty", goes_to: opponent }
cards: { white: team_zero, yellow: none, red: team_zero }
score: "pre_penalty + fouls_received"
ranking:
  aggregate: average
  drop_lowest: { count: 1, not_if: [red] }
  tiebreakers:
    - { label: "Trận cao nhất", expr: "max(matches.team_score)" }
    - { label: "Tổng protection multiplier", expr: "sum(matches.protection) - len(matches.protection)" }
  final_tiebreak: draw
open_questions:
  - id: distribution_value
    used: "0,6 đúng như bảng 3-4 của manual"
    source: "Máy tính Sheets dùng 1/(1+0,6667) = 0,59999; manual là nguồn chính thức (G04)"
  - id: barrier_in_ecosystem
    used: "Còn BARRIER trong ECOSYSTEM → DF = 0,5"
    source: "Manual §3.2; máy tính Sheets bỏ qua luật này"
  - id: sigma_boundaries
    used: "σ ≤ 1 → 1,0; 1 < σ < 10 → 0,6; σ ≥ 10 → 0,5; σ = 0 → 1,0"
    source: "Manual ghi '0 < σ ≤ 1', bỏ sót σ = 0; máy tính Sheets: IFS(σ<=1, 0, ...)"
```

Sai số 0,59999 của máy tính Sheets không bao giờ làm đổi điểm cuối: mọi khoản trước làm tròn là bội của 0,025 trong khi sai số tối đa là 120 × 0,000012 ≈ 0,0014. Ngược lại, máy tính FGC 2026 dùng số thực JS nên **có ca sai thật**: 50 bóng, một robot Zone 1 → JS `ceil(55.00000000000001) = 56`, đúng luật là 55.

Ghi chú: `stddev` trả số thập phân làm tròn ở 28 chữ số có nghĩa (căn bậc hai của `rust_decimal`). Với số bóng nguyên không có tổ hợp nào cho σ đúng bằng 1 hay 10, nên test dùng cặp sát hai phía ranh giới: (0, 0, 2) (σ ≈ 0.943) / (0, 1, 3) (σ ≈ 1.247) và (0, 8, 24) (σ ≈ 9.978) / (0, 4, 23) (σ ≈ 10.033).

## 7. Server mô phỏng

### 7.1 `observe.yaml`

```yaml
field.ext_wildfire:      { count_objects: wildfire, in_target: extinguisher }
side.su_wildfire:        { count_objects: wildfire, in_target: "{side}-suppression-unit", last_released_by: "{side}" }
robot.climb:             { brace_state: { none: NONE, contact: CONTACT, zone1: ZONE_1, zone2: ZONE_2, zone3: ZONE_3 } }
robot.lifted_by_partner: { not_simulated: false }
```

| Phép đo | Hiện thực |
|---|---|
| `count_objects` + `in_target` (+ `last_released_by`) | Giữ vòng theo dõi bóng trong vùng ở `scoring.rs` (`scored_target_by_ball`), bỏ phần cộng điểm; thêm hàm đếm theo target |
| `brace_state` | `brace::brace_state_for_player` (đã có test); thêm nhãn `none/contact/zone1..3` |
| `not_simulated: <giá trị>` | Điền giá trị cố định, ghi tên ô vào `Observation.not_simulated` |

- `{side}` được thay bằng id liên minh.
- Mỗi ô trong `inputs` phải có đúng một ánh xạ; thiếu hoặc thừa thì server từ chối nạp gói đề.

### 7.2 Luồng trong trận

1. Lúc nạp gói: `load(rules.yaml)` và parse `observe.yaml`. Lỗi → không nạp được gói, ghi log rõ tên đề và lỗi.
2. Trong trận, mỗi 0,5 giây: tạo `Observation` tạm, gọi `score`, gửi bảng điểm tạm tính.
3. Ô cấp `robot` được chốt tại tiếng còi hết giờ (giữ hành vi hiện tại `endgame_brace_multipliers`, tránh robot trượt khỏi brace làm đổi điểm); ô `field`/`side` được đo tại `match_finalizes` (hết giờ + `POST_MATCH_SETTLE` 3,5 giây hiện có; manual G21 dùng 5 giây — giữ 3,5 giây để không đổi nhịp trận, ghi trong `open_questions` của observe). Tại `match_finalizes`: tạo `Observation` chính thức, gọi `score`, ghi `result.observation`, `result.scoreSheet`, `result.rulesVersion` vào `MatchCompletionReport`. Giữ `redScore`/`blueScore` để tương thích.
4. Nếu `score` lỗi trong trận: giữ bảng điểm hợp lệ gần nhất, gắn cờ `scoringError`, ghi log, trận vẫn chạy tiếp.

### 7.3 Gỡ bỏ

- `ScoreState.{red_su_score, blue_su_score}`, `apply_score_delta`, `apply_su_multiplier`, `apply_endgame_brace_multipliers`, `brace_multipliers`, nhánh `brace_score_applied`.
- `MatchRuntimeKind::apply_score` và lời gọi `on_trigger_enter`.
- `rules/scoring.rhai`, `rules/penalties.rhai`, mục `scripts.scoring` và `scripts.penalties` trong manifest.
- Trường `points` và `requiresRobotOuttake` trong `manifest.scoring.targets`. Target chỉ còn hình học (`id`, `semanticId`, `area`, `enabled`).

`arena.rhai` và `robot.rhai` giữ nguyên.

### 7.4 Giao thức FGS1

- Section điểm cũ giữ nguyên định dạng, chứa `red`/`blue` lấy từ điểm cuối đã làm tròn; `global` = 0; `breakdown` = các giá trị có `label` ở dạng số nguyên đã làm tròn.
- **Section mới, tag 14 `SCORE_SHEET`** (tag 1–13 đã dùng; 7 là `SEMANTIC_EVENTS`, 9 là `SCORE`): chuỗi JSON UTF-8 của `ScoreSheet` (nhãn, giá trị thập phân dạng chuỗi, `not_simulated`, `scoringError`). Gửi trong baseline và mỗi khi bảng điểm thay đổi.
- Minor version 5 → 6 (`protocol.rs` đang ghi 5; `docs/match-state-protocol.md` ghi 2 là đã lỗi thời, sửa luôn).
- Trang trận đấu có bảng "Chi tiết điểm" đọc section 14; không có thì chỉ hiện điểm cũ.

### 7.5 Manifest và API

- `manifest.json` thêm `"rules": { "scoring": "rules.yaml", "observe": "observe.yaml" }`.
- API endpoint `runtime` trả thêm `rules` (nội dung `rules.yaml` của đề và các đề cha, dạng `{ id: yaml }`) và `observe`.
- **Route công khai mới** `GET /api/rules` (danh sách đề có luật, đọc từ `pkgs/games/index.json`) và `GET /api/rules/:id` (nội dung `rules.yaml`, `text/yaml`). Luật điểm vốn công khai trong game manual. `observe.yaml` và các file `.rhai` vẫn chỉ cấp cho server game.
- API hiện cố định một đề (`PACK_ID = 'fgc-2026'`) cho mô phỏng; route `/api/rules/:id` đọc được mọi đề trong `index.json`.

## 8. Trang trọng tài `/referee/[gameId]`

- Không cần đăng nhập. Tải `/api/rules/:id` cùng các đề cha (`extends`), chạy bản WASM. Trang `/referee` liệt kê các đề từ `/api/rules`.
- Form tự sinh từ `inputs`: cấp field ở trên, hai cột liên minh, mỗi robot một hàng. Mỗi liên minh có bộ đếm lỗi theo `fouls`, mỗi đội có ô mã đội và lựa chọn thẻ theo `cards`.
- Mỗi lần thay đổi thì tính lại ngay. Cột kết quả hiện các giá trị có `label` (giá trị thập phân và giá trị sau làm tròn), điểm phạt, điểm cuối, và điểm từng đội sau thẻ.
- Lỗi biên bản hiện ngay cạnh ô nhập liên quan.
- Bản nháp tự lưu trong `localStorage` theo `gameId` (bọc try/catch). Có nút xoá biên bản.
- Xuất kết quả: in (CSS riêng cho bản in) và tải `bien-ban-<gameId>-<thời gian>.json` (gồm `Observation` và `ScoreSheet`).
- Hai ngôn ngữ vi/en theo Paraglide có sẵn; nhãn ô lấy từ `label`/`label_en`.

## 9. CLI `raso`

```
raso check <thư-mục-đề>...        load + chạy tests/scoring, thoát mã ≠ 0 khi lỗi
raso score <rules.yaml> <biên-bản.yaml> [--json]
raso rank  <rules.yaml> <kết-quả>.json...
```

## 10. Xử lý lỗi

- Lỗi `load` có mã, thông báo tiếng Việt, đường dẫn YAML và số dòng/cột: `RULE_YAML`, `RULE_UNKNOWN_NAME`, `RULE_CYCLE`, `RULE_SCOPE` (dùng tên không thuộc cấp), `RULE_FORBIDDEN` (cú pháp bị cấm), `RULE_EXTENDS`, `RULE_FOUL_REF`.
- Lỗi `score` cũng có mã: `OBS_MISSING`, `OBS_RANGE`, `OBS_CHOICE`, `OBS_ROBOTS`, `EVAL` (kèm tên giá trị, ví dụ chia cho 0 hoặc `lookup` không có khoá), `EVAL_LIMIT`.
- `raso check` coi đề **không có test** là lỗi.

## 11. Kiểm thử

| Lớp | Nội dung |
|---|---|
| Đơn vị `raso-rules` | Mỗi hàm có sẵn và các biên của nó; sắp topo + phát hiện vòng; phạm vi tên theo cấp; số thập phân (`ceil(20 * 1.05) == 21`, `ceil(20 * (1 + 0.05 + 0.10 + 0.20)) == 27`); `goes_to` opponent/self; `team_zero`; `drop_lowest` + `not_if`; `keep_best`; tiebreaker; `extends` + `override` (đề thử trong `crates/raso-rules/tests/fixtures`); hộp cát từ chối `while`, `fn`, `import`, vượt giới hạn phép toán |
| Bộ test của đề | `pkgs/games/fgc-2026/tests/scoring/` và `fgc-2025/...`, mỗi đề ≥ 6 biên bản phủ mọi dòng trong bảng điểm manual, các ranh giới (σ = 1, σ = 10, coop 3/4/5/6, Knockdown bằng nhau), lỗi phạt, thẻ. Kết quả mong đợi tính tay theo manual và đối chiếu máy tính chính thức; ghi chú ca nào máy tính lệch (DF 0,59999) |
| WASM ↔ native | Script Node (vitest trong `web`) chạy cùng bộ biên bản qua bản WASM, so sánh JSON `ScoreSheet` với output `raso score --json` |
| Server | Test tích hợp: dựng `SphereRuntime` FGC 2026, đặt N bóng vào `red-suppression-unit` có `last_outtake_alliance = red`, đặt robot leo zone 2 → `Observation` đúng → `ScoreSheet` đúng. 56 test hiện có phải xanh (trừ các test kiểm tra hàm bị gỡ, được viết lại theo đường mới) |
| Hiệu năng | Benchmark 1.000 bóng p99 không tăng quá 5%; thêm benchmark `score()` FGC 2026 (mục tiêu < 1 ms, đo trên Mac và máy 201) |
| Chống test không thể sai | Với mỗi test quan trọng, chứng minh test đỏ khi cố tình làm hỏng code (ví dụ bỏ `ceil`, đổi `>` thành `>=` ở Knockdown) |

CI thêm job `rules` gồm: `cargo fmt --check`, `cargo test`, `wasm-pack build --target web`, test WASM, `raso check pkgs/games/*`.

## 12. Bố cục code

```
crates/raso-rules/
  Cargo.toml              features: cli, wasm
  src/lib.rs              load / score / rank
  src/schema.rs           kiểu dữ liệu YAML (serde)
  src/extends.rs          kế thừa + override
  src/expr.rs             Engine Rhai (decimal, hộp cát, hàm có sẵn)
  src/graph.rs            phạm vi tên + sắp topo
  src/eval.rs             tính theo cấp robot → side → field, phạt, thẻ, làm tròn
  src/rank.rs             xếp hạng
  src/error.rs            mã lỗi + thông báo tiếng Việt
  src/bin/raso.rs         CLI
  src/wasm.rs             wasm-bindgen
  tests/                  test đơn vị + fixtures
pkgs/games/fgc-2026/{rules.yaml, observe.yaml, tests/scoring/*.yaml}
pkgs/games/fgc-2025/{manifest.json, rules.yaml, tests/scoring/*.yaml}   # chưa có sân 3D
server/src/game/scoring_bridge.rs     Observation từ SphereRuntime + gọi raso-rules
web/src/routes/(public)/referee/[gameId]/+page.svelte (+ components)
```

`pkgs/games/fgc-2025/manifest.json` chỉ có `id`, `name`, `version`, `rules`. API và server bỏ qua đề không có `field`. Trang trọng tài dùng được mọi đề có `rules`.

## 13. Rủi ro và giả định cần kiểm chứng sớm

| Giả định | Cách kiểm chứng (task đầu của kế hoạch) | Nếu sai |
|---|---|---|
| ~~`no_float` + `decimal`~~ → tiền xử lý chữ số thành `__d("…")` | **Đã kiểm chứng 15/10/2026:** `ceil(20*1.05)=21`, `1/3=0,333…`; server vẫn qua 56 test khi Rhai bật thêm `decimal`, `sync` | — |
| Rhai + `rust_decimal` biên dịch được sang `wasm32-unknown-unknown` | **Đã kiểm chứng:** build được khi bật feature `wasm-bindgen` của Rhai + `getrandom 0.2` feature `js` cho target wasm32 | — |
| Tắt `while`/`loop`/`for`/`fn` bằng `Engine::disable_symbol` là đủ | Test hộp cát | Dùng `Engine::on_parse_token` để chặn |
| `score()` FGC 2026 < 1 ms | Benchmark | Gọi mỗi 1 giây thay vì 0,5 giây |

## Phụ lục A — Bảng tra nguồn FGC 2026

| Mục | Giá trị | Nguồn |
|---|---|---|
| WILDFIRE trong SU / EXT | 1 điểm | Bảng 3-1 |
| Hệ số leo | 1 + Σ (0,05 / 0,10 / 0,20 / 0,30) | Bảng 3-2; máy tính chính thức |
| Partner climb | 25 điểm / robot được nâng | Bảng 3-3 |
| Knockdown | 10 điểm | Bảng 3-4 |
| Coopertition | 4/5/6 robot Zone 3 → 10/25/40 | Bảng 3-5 |
| Lỗi nhẹ / nặng | 5% / 10% điểm trước phạt của bên phạm lỗi → cộng cho đối thủ | Bảng 4-1 |
| Thẻ trắng / đỏ | Đội nhận 0 điểm trận (vòng xếp hạng) | Bảng 4-1 |
| Xếp hạng | Trung bình, bỏ 1 trận thấp nhất (trừ trận thẻ đỏ) | M21, §6.3 |

## Phụ lục B — Bảng tra nguồn FGC 2025

| Mục | Giá trị | Nguồn |
|---|---|---|
| BARRIER / BIODIVERSITY UNIT | 1 điểm | Bảng 3-3 |
| Distribution factor | 1,0 / 0,6 / 0,5 theo σ; còn BARRIER → 0,5 | Bảng 3-4, §3.2 |
| Protection multiplier | 1 + Σ (0,125 / 0,25 / 0,375 / 0,5) | Bảng 3-5; máy tính Sheets `=1+sum(...)` |
| Coopertition | 5/6 robot treo → 15/30 | Bảng 3-6 |
| Công thức | `roundup((B + U × DF) × PM + C)` | §3.6; máy tính Sheets |
| Phạt, thẻ, xếp hạng | như 2026; tiebreak 2 = tổng protection multiplier | Bảng 4-1, §6.3 |

## Phụ lục C — FAnRoC SeaHeal 2026–2027 (tạm gác)

Thể lệ (https://docs.google.com/document/d/1wrrRwXzl-obWM3iTIDcgyFZucQXG6J-DjLq9QIo0Oww) có 6 chỗ mâu thuẫn, cần BTC trả lời trước khi viết `rules.yaml`:

1. San hô 4 điểm/quả (mục B) hay 1 điểm (bảng điểm dòng 2).
2. Điểm san hô cộng cho cả hai liên minh hay chỉ liên minh ít điểm rác hơn.
3. 1 khối + đủ 2 robot về chuồng: ×1,75 (mục C) hay ×1,5 (bảng điểm dòng 3).
4. Bóng san hô màu xanh lá hay trắng.
5. Phạt bắn nhầm trụ không ghi số điểm.
6. Robot A là robot bị nhốt (dòng 48) hay robot đi mở cửa (dòng 75, 135).

Định dạng v1 đủ để viết đề này: `min()` cho giỏ bập bênh, bảng tra 2 chiều cho hệ số năng lượng, `goes_to: self` với công thức `score` nhân cả phạt.

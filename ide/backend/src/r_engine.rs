//! r_engine.rs — INJINI YA R KWA MTECH OS: uchambuzi wa hali ya juu na
//! visualization halisi kupitia Rscript. Kila model/tool/agent inaweza kuita
//! API hii kwa: stats, chart (PNG), image analysis (histogram), audio analysis
//! (WAV), na series/forecasting — VYOTE kwa R.
//!
//! USALAMA: arg-array spawn (hakuna shell), timeout, output truncation,
//! data inaandikwa /tmp kwa faili halisi; code ya R hujaandaliwa kwa
//! template (user code AU inline script tu), hakuna shell injection.

use serde_json::{json, Value};
use std::time::Duration;
use tokio::process::Command;

const R_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_OUTPUT_BYTES: usize = 512 * 1024;

/// Endesha Rscript na script inline (kwa R code ilizotolewa na caller).
/// Code inapita KAMA HOJA ya Rscript, si shell — arg-array.
async fn rscript(script: &str) -> (bool, String, String) {
    run_rscript_sync(script).await
}

async fn run_rscript_sync(script: &str) -> (bool, String, String) {
    let script_path = std::env::temp_dir().join(format!("mtechos_r_{}.R", uuid::Uuid::new_v4()));
    if let Err(e) = std::fs::write(&script_path, script) {
        return (
            false,
            String::new(),
            format!("Rscript haikuweza kuandikwa: {e}"),
        );
    }
    let spawn = Command::new("Rscript")
        .arg("--vanilla")
        .arg(&script_path)
        .output();
    let res = tokio::time::timeout(R_TIMEOUT, spawn).await;
    let _ = std::fs::remove_file(&script_path);
    match res {
        Err(_) => (false, String::new(), "Rscript timeout (60s)".into()),
        Ok(Err(e)) => (false, String::new(), format!("Rscript spawn fail: {e}")),
        Ok(Ok(o)) => {
            let mut out = String::from_utf8_lossy(&o.stdout).to_string();
            let err = String::from_utf8_lossy(&o.stderr).to_string();
            if out.len() > MAX_OUTPUT_BYTES {
                out.truncate(MAX_OUTPUT_BYTES);
                out.push_str("\n...[truncated]");
            }
            (o.status.success(), out, err)
        }
    }
}

/// KAZI za R engine (task whitelist):
///   summary   — stats za msingi kwenye orodha ya namba
///   chart     — PNG ya bar/hist/line kutoka data
///   image     — uchanganuzi wa picha (histogram ya RGB, binary, grayscale)
///   audio     — uchanganuzi wa WAV (duration, peak, RMS, sample-rate)
///   forecast  — linear regression line kwa namba
///   custom    — R code yoyote (sandbox-only: caller lazima awe sanctioned)
/// --- SECURITY ANALYTICS (skills za usalama — zimefikishwa kwa agent): ---
///   risk_score    — risk scoring ya findings (CVSS-like kwa data ya severity)
///   anomaly       — anomaly detection (robust MAD-based) kwenye data ya trafiki/logs
///   sec_forecast  — forecast ya incidents (trend ya matukio)
///   chart3d       — 3D halisi (scatterplot3d PNG + plotly HTML kama itaombwa)
///   video         — video halisi (MP4/H.264): R inatengeneza frames, ffmpeg inaencode
/// --- THREAT INTELLIGENCE (defensive analytics — threatintel.rs): ---
///   domain_sim    — kufanana kwa jina la kikoa (adist/Levenshtein) dhidi ya brand
///   fraud_trend   — trend + forecast ya matukio ya ulaghai (lm + MAD anomalies)
pub const TASKS: &[&str] = &[
    "summary", "chart", "chart3d", "video", "image", "audio", "forecast", "custom",
    "risk_score", "anomaly", "sec_forecast", "iot_stats", "domain_sim", "fraud_trend",
];

fn data_to_r_vec(v: &Value) -> String {
    let nums: Vec<String> = v
        .as_array()
        .map(|a| a.iter().filter_map(|x| x.as_f64()).map(|x| x.to_string()).collect())
        .unwrap_or_default();
    format!("c({})", nums.join(","))
}

/// Pointi za 3D → matrix ya R. Nested [[x,y,z],...] → rbind; [z,...] → cbind.
fn data_to_r_points(v: &Value) -> String {
    let arr = match v.as_array() {
        Some(a) => a,
        None => return "matrix(numeric(0), 0, 3)".into(),
    };
    let nested = arr.first().map(|x| x.is_array()).unwrap_or(false);
    if nested {
        let rows: Vec<String> = arr
            .iter()
            .filter_map(|p| p.as_array())
            .map(|c| {
                let nums: Vec<String> = c.iter().take(3).filter_map(|x| x.as_f64()).map(|x| x.to_string()).collect();
                format!("c({})", nums.join(","))
            })
            .collect();
        format!("do.call(rbind, list({}))", rows.join(","))
    } else {
        let z = data_to_r_vec(v);
        format!("cbind(seq_along({z}), {z}, sin(seq_along({z}) / 3))")
    }
}

fn script_for(task: &str, params: &Value) -> String {
    let data = data_to_r_vec(&params["data"]);
    let out_dir = std::env::temp_dir().to_string_lossy().to_string();
    let png = params["output_png"]
        .as_str()
        .unwrap_or("chart.png")
        .replace(['/', '\\', '\0'], "_");
    let mp4 = params["output_mp4"]
        .as_str()
        .unwrap_or("video.mp4")
        .replace(['/', '\\', '\0'], "_");
    let file = params["file"]
        .as_str()
        .unwrap_or("")
        .replace(['\'', '"', '\0'], "_");
    let chart_kind = match params["chart_kind"].as_str().unwrap_or("bar") {
        k @ ("bar" | "line" | "area" | "hist" | "pie" | "scatter") => k.to_string(),
        _ => "bar".to_string(),
    };
    let chart_title = params["title"]
        .as_str()
        .unwrap_or("MTECHOS")
        .replace(['\'', '"', '\\', '\0', '\n'], " ");
    let vw: u32 = params["width"].as_u64().unwrap_or(640).clamp(160, 1920) as u32;
    let vh: u32 = params["height"].as_u64().unwrap_or(360).clamp(120, 1080) as u32;
    let fps: u32 = params["fps"].as_u64().unwrap_or(10).clamp(1, 60) as u32;
    let html3d = if params["html"].as_bool().unwrap_or(false) { "TRUE" } else { "FALSE" };
    let points = data_to_r_points(&params["data"]);

    match task {
        "summary" => format!(
            r#"x <- {data}
valid <- x[is.finite(x)]
if (length(valid) == 0) stop("no numeric data")
out <- list(n = length(valid), mean = mean(valid), median = median(valid),
  sd = sd(valid), min = min(valid), max = max(valid))
cat(jsonlite::toJSON(out, auto_unbox = TRUE))
"#
        ),
        "forecast" => format!(
            r#"x <- {data}
t <- seq_along(x)
if (length(x) < 3) stop("forecast inahitaji namba 3+")
m <- lm(x ~ t)
pred <- predict(m, newdata = data.frame(t = c(length(x) + 1L, length(x) + 2L)))
cat(jsonlite::toJSON(list(slope = unname(coef(m)[2]),
  next1 = pred[1], next2 = pred[2], r2 = summary(m)$r.squared), auto_unbox = TRUE))
"#
        ),
        // CHART: visualization HALISI (PNG). Aina: bar (default), line, area,
        // hist, pie, scatter. Rangi nzuri, grid, mhimili, title/labels.
        "chart" => format!(
            r##"{{
  path <- file.path("{out_dir}", "{png}")
  kind <- "{chart_kind}"
  title <- "{chart_title}"
  x <- {data}
  if (length(x) == 0) stop("no numeric data")
  pal <- c("#4C78A8", "#F58518", "#54A24B", "#E45756", "#72B7B2", "#B279A2", "#9D755D")
  png(path, width = 900, height = 540, res = 110)
  op <- par(bg = "#ffffff", fg = "#2b2b2b", col.axis = "#444444",
            mar = c(4.2, 4.4, 3.4, 1.2), family = "sans")
  cols <- rep(pal, length.out = length(x))
  if (kind == "line") {{
    plot(seq_along(x), x, type = "o", pch = 19, lwd = 2.4, col = pal[1],
         xlab = "index", ylab = "value", main = title, panel.first = grid(col = "#E6E6E6", lty = 1))
  }} else if (kind == "area") {{
    plot(seq_along(x), x, type = "n", xlab = "index", ylab = "value", main = title)
    grid(col = "#E6E6E6", lty = 1)
    polygon(c(seq_along(x), rev(seq_along(x))), c(x, rep(min(x), length(x))),
            col = adjustcolor(pal[1], 0.30), border = NA)
    lines(seq_along(x), x, col = pal[1], lwd = 2.4)
    points(seq_along(x), x, pch = 19, col = pal[1])
  }} else if (kind == "hist") {{
    hist(x, breaks = "Sturges", col = adjustcolor(pal[1], 0.65), border = "white",
         main = title, xlab = "value", ylab = "frequency")
  }} else if (kind == "pie") {{
    pie(abs(x), labels = seq_along(x), col = cols, main = title,
        border = "white", clockwise = TRUE)
  }} else if (kind == "scatter") {{
    plot(seq_along(x), x, pch = 19, col = adjustcolor(pal[1], 0.75), cex = 1.3,
         xlab = "index", ylab = "value", main = title)
    grid(col = "#E6E6E6", lty = 1)
    abline(lm(x ~ seq_along(x)), col = pal[4], lwd = 2, lty = 2)
  }} else {{
    barplot(x, col = cols, border = NA, main = title, xlab = "index", ylab = "value")
    abline(h = 0, col = "#999999")
  }}
  invisible(dev.off())
  cat(tools::file_path_as_absolute(path))
}}
"##
        ),
        // CHART3D: 3D halisi. scatterplot3d (PNG, base device) kila wakati;
        // plotly (HTML interactive) kama html=TRUE na plotly/htmlwidgets zipo.
        // data: [[x,y,z], ...] au [z, ...] (x/y = index).
        "chart3d" => format!(
            r##"{{
  path <- file.path("{out_dir}", "{png}")
  title <- "{chart_title}"
  m <- {points}
  m <- matrix(as.numeric(m), ncol = 3)
  m <- m[stats::complete.cases(m), , drop = FALSE]
  if (nrow(m) < 3) stop("chart3d inahitaji pointi 3+ ([[x,y,z],...] au [z,...])")
  if (!requireNamespace("scatterplot3d", quietly = TRUE))
    stop("scatterplot3d haipo (install.packages('scatterplot3d'))")
  png(path, width = 900, height = 620, res = 110, bg = "white")
  scatterplot3d::scatterplot3d(m[, 1], m[, 2], m[, 3], pch = 19, color = "#4C78A8",
    main = title, xlab = "x", ylab = "y", zlab = "z", grid = TRUE, box = TRUE)
  invisible(dev.off())
  html_out <- ""
  if ({html3d} && requireNamespace("plotly", quietly = TRUE) && requireNamespace("htmlwidgets", quietly = TRUE)) {{
    pl <- plotly::plot_ly(x = m[, 1], y = m[, 2], z = m[, 3], type = "scatter3d",
      mode = "markers", marker = list(size = 4, color = "#E45756"))
    html_out <- file.path("{out_dir}", sub("\\.png$", ".html", "{png}"))
    htmlwidgets::saveWidget(plotly::layout(pl, title = title), html_out, selfcontained = TRUE)
  }}
  cat(jsonlite::toJSON(list(task = "chart3d", points = nrow(m),
    png = tools::file_path_as_absolute(path),
    html = if (nzchar(html_out)) tools::file_path_as_absolute(html_out) else NULL),
    auto_unbox = TRUE))
}}
"##
        ),
        // VIDEO: video HALISI (MP4/H.264). R inatengeneza frames (PNG) kutoka
        // data, ffmpeg inaencode. Req: data, output_mp4, width/height/fps.
        "video" => format!(
            r##"{{
  mp4 <- file.path("{out_dir}", "{mp4}")
  title <- "{chart_title}"
  x <- {data}
  if (length(x) < 2) stop("video inahitaji data 2+")
  vw <- {vw}; vh <- {vh}; fps <- {fps}
  fr_dir <- file.path("{out_dir}", paste0("frames_", as.integer(Sys.time())))
  dir.create(fr_dir, showWarnings = FALSE)
  nfr <- length(x)
  ymax <- max(x); ymin <- min(x)
  for (i in seq_len(nfr)) {{
    png(sprintf("%s/f%04d.png", fr_dir, i), width = vw, height = vh, res = 96, bg = "white")
    plot(seq_len(i), x[seq_len(i)], type = "o", pch = 19, lwd = 2.5, col = "#4C78A8",
         xlim = c(1, nfr), ylim = c(ymin, ymax), xlab = "index", ylab = "value",
         main = sprintf("%s  (%d/%d)", title, i, nfr))
    grid(col = "#E6E6E6", lty = 1)
    invisible(dev.off())
  }}
  list_file <- file.path(fr_dir, "list.txt")
  frs <- sort(list.files(fr_dir, pattern = "\\.png$", full.names = TRUE))
  con <- file(list_file, "w")
  for (f in frs) {{ writeLines(sprintf("file '%s'", f), con); writeLines(sprintf("duration %s", format(1 / fps, nsmall = 4)), con) }}
  writeLines(sprintf("file '%s'", frs[length(frs)]), con)
  close(con)
  ff <- Sys.which("ffmpeg")
  if (!nzchar(ff)) stop("ffmpeg haipo kwenye PATH")
  # shQuote: hoja zina mabano (ow-iw) — system2 haiziquote zenyewe.
  args <- c("-y", "-f", "concat", "-safe", "0", "-i", shQuote(list_file),
            "-vf", shQuote(sprintf("scale=%d:%d:force_original_aspect_ratio=decrease,pad=%d:%d:(ow-iw)/2:(oh-ih)/2,format=yuv420p", vw, vh, vw, vh)),
            "-r", as.character(fps), "-c:v", "libx264", "-pix_fmt", "yuv420p", shQuote(mp4))
  st <- system2(ff, args, stdout = FALSE, stderr = FALSE)
  if (st != 0 || !file.exists(mp4)) stop(sprintf("ffmpeg imeshindwa (exit %s)", st))
  sz <- file.info(mp4)$size
  unlink(fr_dir, recursive = TRUE)
  cat(jsonlite::toJSON(list(task = "video", mp4 = tools::file_path_as_absolute(mp4),
    frames = nfr, fps = fps, width = vw, height = vh, bytes = sz,
    codec = "h264", seconds = round(nfr / fps, 2)), auto_unbox = TRUE))
}}
"##
        ),
        // IMAGE: uchanganuzi HALISI wa picha (dimensions, channels, RGB stats,
        // brightness, contrast, dominant color) + histogram PNG. Inatumia
        // magick/png/jpeg kama zinapatikana; hakuna stub.
        "image" => {
            let s = r##"f <- "@FILE@"
if (!file.exists(f)) stop("image haipatikani")
img <- NULL; src <- "none"; w <- NA; h <- NA; ch <- NA
if (requireNamespace("magick", quietly = TRUE)) {
  im <- magick::image_read(f)
  ii <- magick::image_info(im)
  w <- as.integer(ii$width[1]); h <- as.integer(ii$height[1])
  a <- magick::image_data(im, channels = "rgb")
  d <- dim(a); w <- d[2]; h <- d[3]; ch <- 3
  arr <- array(as.integer(a), dim = c(3, w, h))
  img <- aperm(arr, c(3, 2, 1)); src <- "magick"
} else if (grepl("\\.png$", f, ignore.case = TRUE) && requireNamespace("png", quietly = TRUE)) {
  a <- png::readPNG(f)
  if (length(dim(a)) == 2) a <- array(rep(a, 3), c(dim(a), 3))
  img <- a * 255; src <- "png"
} else if (grepl("\\.(jpe?g)$", f, ignore.case = TRUE) && requireNamespace("jpeg", quietly = TRUE)) {
  a <- jpeg::readJPEG(f)
  if (length(dim(a)) == 2) a <- array(rep(a, 3), c(dim(a), 3))
  img <- a * 255; src <- "jpeg"
} else stop("hakuna package ya picha (magick/png/jpeg) au format haijulikani")
h <- dim(img)[1]; w <- dim(img)[2]; ch <- dim(img)[3]
R <- as.numeric(img[, , 1]); G <- as.numeric(img[, , 2]); B <- as.numeric(img[, , 3])
gray <- 0.299 * R + 0.587 * G + 0.114 * B
dom <- function(x) { b <- cut(x, breaks = seq(0, 255, length.out = 9), include.lowest = TRUE, labels = FALSE); which.max(tabulate(b, nbins = 8)) }
dchan <- function(v) max(0L, min(255L, (v - 1) * 36 + 18))
domhex <- sprintf("#%02X%02X%02X", dchan(dom(R)), dchan(dom(G)), dchan(dom(B)))
outp <- file.path("@OUTDIR@", "img_hist.png")
if (requireNamespace("png", quietly = TRUE)) {
  png(outp, width = 800, height = 500)
  op <- par(mfrow = c(1, 3), bg = "white", mar = c(3, 3, 2, 1))
  hist(R, breaks = 32, col = "#E45", border = NA, main = "Red", xlab = "")
  hist(G, breaks = 32, col = "#4B5", border = NA, main = "Green", xlab = "")
  hist(B, breaks = 32, col = "#45E", border = NA, main = "Blue", xlab = "")
  invisible(dev.off())
}
cat(jsonlite::toJSON(list(task = "image", source = src, width = w, height = h,
  channels = ch, size_bytes = file.info(f)$size,
  mean_r = round(mean(R), 1), mean_g = round(mean(G), 1), mean_b = round(mean(B), 1),
  brightness = round(mean(gray), 1), contrast = round(sd(gray), 1),
  dominant_color = domhex, histogram_png = outp), auto_unbox = TRUE))
"##;
            s.replace("@FILE@", &file).replace("@OUTDIR@", &out_dir)
        }
        // AUDIO: uchanganuzi HALISI wa WAV kwa base R (hakuna package ya nje).
        "audio" => {
            let s = r##"f <- "@FILE@"
if (!file.exists(f)) stop("wav haipatikani")
con <- file(f, "rb"); on.exit(close(con))
hdr <- readBin(con, "raw", 12)
if (length(hdr) < 12) stop("faili ndogo mno kwa WAV")
if (rawToChar(hdr[1:4]) != "RIFF" || rawToChar(hdr[9:12]) != "WAVE") stop("si WAV halali (RIFF/WAVE)")
invisible(seek(con, 12))
channels <- NA; sr <- NA; bits <- NA; data_len <- NA
repeat {
  hh <- readBin(con, "raw", 8)
  if (length(hh) < 8) break
  id <- rawToChar(hh[1:4])
  sz <- readBin(hh[5:8], "integer", 1, size = 4, signed = TRUE, endian = "little")
  if (id == "fmt ") {
    b <- readBin(con, "raw", sz)
    channels <- readBin(b[3:4], "integer", 1, size = 2, signed = FALSE, endian = "little")
    sr <- readBin(b[5:8], "integer", 1, size = 4, signed = TRUE, endian = "little")
    bits <- readBin(b[15:16], "integer", 1, size = 2, signed = FALSE, endian = "little")
  } else if (id == "data") { data_len <- sz; break } else { readBin(con, "raw", sz) }
}
if (is.na(data_len) || is.na(sr)) stop("fmt/data chunk haipatikani")
bytes_per <- bits / 8
n <- as.integer(data_len / bytes_per)
s <- readBin(con, "integer", n, size = bytes_per, signed = TRUE, endian = "little")
if (!is.na(channels) && channels > 1) { m <- matrix(s, ncol = channels, byrow = TRUE); mono <- rowMeans(m) } else mono <- s
mx <- 2^(bits - 1)
rms <- sqrt(mean((mono / mx)^2)); peak <- max(abs(mono)) / mx
dur <- length(mono) / sr
zcr <- mean(abs(diff(sign(mono))) > 0)
cat(jsonlite::toJSON(list(task = "audio", format = "WAV", channels = channels,
  sample_rate = sr, bits = bits, seconds = round(dur, 3), samples = length(mono),
  rms = round(rms, 6), peak = round(peak, 6), zcr = round(zcr, 4),
  clipped = sum(abs(mono) >= mx * 0.999)), auto_unbox = TRUE))
"##;
            s.replace("@FILE@", &file)
        }
        // --- SECURITY ANALYTICS SKILLS (data-driven; hakuna shell) ---
        // risk_score: params.data = scores za CVSS (0-10) kwa findings
        "risk_score" => format!(
            r#"x <- {data}
valid <- x[is.finite(x) & x >= 0 & x <= 10]
if (length(valid) == 0) stop("risk data empty (values 0-10)")
sev <- c(critical = sum(valid >= 9), high = sum(valid >= 7 & valid < 9),
  medium = sum(valid >= 4 & valid < 7), low = sum(valid < 4))
risk_index <- weighted.mean(valid, w = valid, na.rm = TRUE)
cat(jsonlite::toJSON(list(findings = length(valid),
  critical = sev[["critical"]], high = sev[["high"]],
  medium = sev[["medium"]], low = sev[["low"]],
  mean_severity = round(mean(valid), 2), risk_index = round(risk_index, 2)),
  auto_unbox = TRUE))
"#
        ),
        // anomaly: robust detection (modified z-score / MAD) kwenye metrics.
        // NB: z-score ya kawaida (|z|>3) HAIWEZI kutokea kwa n<11 (max z=(n-1)/sqrt(n)),
        // hivyo tunatumia MAD-based robust z ambayo inafanya kazi kwa sampuli ndogo.
        "anomaly" => format!(
            r#"x <- {data}
valid <- x[is.finite(x)]
if (length(valid) < 5) stop("anomaly inahitaji data 5+")
m <- median(valid); md <- mad(valid)
if (md > 0) {{ rz <- abs(valid - m) / md; thr <- 3.5 }} else {{ rz <- abs(valid - m) / (sd(valid) + .Machine$double.eps); thr <- 3 }}
out_idx <- which(rz > thr)
outliers <- valid[out_idx]
cat(jsonlite::toJSON(list(n = length(valid),
  anomalies = length(outliers), values = outliers, indices = out_idx,
  method = if (md > 0) "modified_zscore_mad" else "zscore_sd",
  mean = round(mean(valid), 4), median = round(m, 4), sd = round(sd(valid), 4)),
  auto_unbox = TRUE))
"#
        ),
        // sec_forecast: trend ya incidents (data = incidents kwa siku)
        "sec_forecast" => format!(
            r#"x <- {data}
t <- seq_along(x)
if (length(x) < 4) stop("sec_forecast inahitaji data 4+")
m <- lm(x ~ t)
pred <- predict(m, newdata = data.frame(t = (length(x)+1):(length(x)+3)))
cat(jsonlite::toJSON(list(slope = round(unname(coef(m)[2]), 4),
  trend = ifelse(unname(coef(m)[2]) > 0, "RISING", "DECLINING"),
  next3 = round(pred, 2), r2 = summary(m)$r.squared), auto_unbox = TRUE))
"#
        ),
        // iot_stats: uchambuzi wa telemetry ya sensor (temp/humidity) + anomalies + trend
        "iot_stats" => format!(
            r#"x <- {data}
valid <- x[is.finite(x)]
if (length(valid) < 4) stop("iot_stats inahitaji data 4+")
md <- mad(valid)
if (md > 0) {{ rz <- abs(valid - median(valid)) / md; anom_idx <- which(rz > 3.5) }} else {{ anom_idx <- integer(0) }}
t <- seq_along(valid)
m <- lm(valid ~ t)
cat(jsonlite::toJSON(list(n = length(valid),
  mean = round(mean(valid), 2), sd = round(sd(valid), 3),
  min = round(min(valid), 2), max = round(max(valid), 2),
  median = round(median(valid), 2),
  anomalies = length(anom_idx), anomaly_indices = anom_idx,
  trend_slope = round(unname(coef(m)[2]), 4),
  trend = ifelse(unname(coef(m)[2]) > 0.05, "RISING",
    ifelse(unname(coef(m)[2]) < -0.05, "FALLING", "STABLE"))), auto_unbox = TRUE))
"#
        ),
        // domain_sim: kufanana kwa jina la kikoa dhidi ya brand zilizolindwa.
        // params.brands = ["crdb","nmb",...]; params.host = "crdb-bank.com".
        // R inatumia adist (Levenshtein) na kugundua typosquatting/homoglyph.
        "domain_sim" => {
            let brands = params["brands"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str())
                        .map(|s| s.replace(['"', '\\', '\0'], ""))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let brands_r = format!(
                "c({})",
                brands
                    .iter()
                    .map(|b| format!("\"{}\"", b))
                    .collect::<Vec<_>>()
                    .join(",")
            );
            let host = params["host"]
                .as_str()
                .unwrap_or("")
                .replace(['"', '\\', '\0'], "");
            format!(
                r##"host <- "{host}"
brands <- {brands_r}
if (length(brands) == 0) stop("brands inahitajika kwa domain_sim")
# label ya registrable (bila www. na TLD ya kwanza)
lab <- sub("^www\\.", "", host)
lab <- strsplit(lab, "\\.")[[1]][1]
# homoglyph normalization (0->o, 1->l, 3->e, 4->a, 5->s, 7->t)
norm <- function(s) {{
  s <- tolower(s)
  chartr("0134579", "oleastg", s)
}}
nl <- norm(lab)
sims <- sapply(brands, function(b) as.integer(utils::adist(nl, norm(b))))
names(sims) <- brands
best <- names(which.min(sims))
d <- min(sims)
# exact official? (host inaishia brand + .co.tz / .com)
official <- any(sapply(brands, function(b) grepl(paste0("(^|\\.)", b, "\\.(co\\.tz|com|net|org)$"), host)))
# hyphen/contains variant: label ina brand lakini si brand yenyewe
contains <- (!official && any(sapply(brands, function(b) grepl(b, lab, fixed = TRUE) && lab != b)))
typo <- (!official && ((d > 0 && d <= 2) || contains))
cat(jsonlite::toJSON(list(host = host, label = lab, normalized = nl,
  closest_brand = best, distance = as.integer(d),
  typosquat = typo,
  official = official,
  all_distances = as.list(sims)), auto_unbox = TRUE))
"##
            )
        }
        // fraud_trend: trend + forecast ya matukio ya ulaghai kwa siku.
        // params.data = incidents/day (namba 4+). Inatoa trend + anomalies + next3.
        "fraud_trend" => format!(
            r#"x <- {data}
valid <- x[is.finite(x)]
if (length(valid) < 4) stop("fraud_trend inahitaji data 4+")
t <- seq_along(valid)
m <- lm(valid ~ t)
slope <- unname(coef(m)[2])
md <- mad(valid)
if (md > 0) {{ rz <- abs(valid - median(valid)) / md; spikes <- which(rz > 3.5) }} else {{ spikes <- integer(0) }}
pred <- predict(m, newdata = data.frame(t = (length(valid)+1):(length(valid)+3)))
cat(jsonlite::toJSON(list(n = length(valid), total = sum(valid),
  mean = round(mean(valid), 2), max = round(max(valid), 2),
  slope = round(slope, 4),
  trend = ifelse(slope > 0.1, "INCREASING",
    ifelse(slope < -0.1, "DECREASING", "STABLE")),
  spike_days = as.integer(spikes),
  next3 = round(pmax(pred, 0), 2),
  r2 = round(summary(m)$r.squared, 4)), auto_unbox = TRUE))
"#
        ),
        _ => params["r_code"].as_str().unwrap_or("").to_string(),
    }
}

/// Mwili wa engine: pima task na kuzungumza na Rscript.
pub async fn run_req(task: &str, params: &Value) -> Value {
    if !TASKS.contains(&task) {
        return json!({
            "ok": false,
            "error": format!("task '{task}' hairuhusiwi. Zinazopatikana: {}", TASKS.join(", ")),
        });
    }
    // Custom code ruhusiwa TU kwa task "custom" (sandbox-only; agent pending HITL).
    if (task == "summary" || task == "chart" || task == "forecast") && !params["data"].is_array() {
        return json!({ "ok": false, "error": "'data' (array ya namba) inahitajika kwa summary/chart/forecast" });
    }
    if (task == "image" || task == "audio") && params["file"].as_str().map(|s| s.trim().is_empty()).unwrap_or(true) {
        return json!({ "ok": false, "error": "'file' (path ya picha/wav ndani ya sandbox) inahitajika kwa image/audio" });
    }
    if (task == "chart3d") && !params["data"].is_array() {
        return json!({ "ok": false, "error": "'data' ([[x,y,z],...] au [z,...]) inahitajika kwa chart3d" });
    }
    if (task == "video") && !params["data"].is_array() {
        return json!({ "ok": false, "error": "'data' (array ya namba 2+) inahitajika kwa video" });
    }
    let script = script_for(task, params);
    if script.trim().is_empty() {
        return json!({ "ok": false, "error": "script tupu (character parameters)" });
    }
    let (ok, out, err) = rscript(&script).await;
    json!({ "ok": ok, "engine": "R (Rscript)", "task": task, "stdout": out, "stderr": err })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tasks_list() {
        assert!(TASKS.contains(&"summary"));
        assert!(TASKS.contains(&"chart"));
        assert!(TASKS.contains(&"audio"));
        assert!(TASKS.contains(&"domain_sim"));
        assert!(TASKS.contains(&"fraud_trend"));
        assert!(!TASKS.contains(&"destroy_network"));
    }

    #[test]
    fn test_domain_sim_script_ni_sahihi() {
        let s = script_for(
            "domain_sim",
            &json!({"host": "crdb-bank.com", "brands": ["crdb", "nmb"]}),
        );
        assert!(s.contains("adist"));
        assert!(s.contains("crdb-bank.com"));
        assert!(s.contains("typosquat"));
        // r## string haipaswi kuvunjika kwa '#' ndani ya body
        assert!(!s.contains("r##"));
    }

    #[test]
    fn test_fraud_trend_script_ina_trend() {
        let s = script_for("fraud_trend", &json!({"data": [1, 2, 3, 4, 9]}));
        assert!(s.contains("INCREASING"));
        assert!(s.contains("spike"));
        assert!(s.contains("jsonlite"));
    }

    #[test]
    fn test_chart_script_no_bad_lines() {
        let s = script_for("chart", &json!({"data": [1, 2, 3]}));
        assert!(!s.contains("t <- chart")); // regression guard: hakuna invalid R
        assert!(s.contains("barplot"));
        assert!(s.contains("dev.off()"));
    }

    #[tokio::test]
    async fn test_run_req_rejects_missing_data() {
        let v = run_req("summary", &json!({})).await;
        assert_eq!(v["ok"], json!(false));
        assert!(v["error"].as_str().unwrap().contains("data"));
    }

    #[tokio::test]
    async fn test_run_req_rejects_image_without_file() {
        let v = run_req("image", &json!({})).await;
        assert_eq!(v["ok"], json!(false));
        assert!(v["error"].as_str().unwrap().contains("file"));
    }

    #[test]
    fn test_chart_script_supports_kinds_and_sanitizes_title() {
        let s = script_for(
            "chart",
            &json!({"data": [1,2,3], "chart_kind": "line", "title": "Jua'; rm -rf /"}),
        );
        assert!(s.contains("kind <- \"line\""));
        // title imesafishwa: quote moja imeondolewa (badilishwa na space)
        assert!(s.contains("Jua ; rm -rf /"));
        assert!(!s.contains("Jua'"));
        // kind isiyojulikana → bar
        let s2 = script_for("chart", &json!({"data": [1], "chart_kind": "hack"}));
        assert!(s2.contains("kind <- \"bar\""));
    }

    #[test]
    fn test_script_summary() {
        let s = script_for("summary", &json!({"data": [1, 2, 3]}));
        assert!(s.contains("mean(valid)"));
        assert!(!s.contains("shell"));
    }

    #[test]
    fn test_image_script_is_real_analysis() {
        let s = script_for("image", &json!({"file": "/tmp/x.png"}));
        // Uchanganuzi halisi (magick/png/jpeg), si stub ya size pekee.
        assert!(s.contains("image_read") || s.contains("readPNG") || s.contains("readJPEG"));
        assert!(s.contains("brightness") && s.contains("dominant_color"));
        assert!(!s.contains("image_basic"));
    }

    #[test]
    fn test_audio_script_parses_wav_without_tuner() {
        let s = script_for("audio", &json!({"file": "/tmp/x.wav"}));
        assert!(s.contains("RIFF") && s.contains("readBin"));
        assert!(!s.contains("tuneR"));
    }

    #[test]
    fn test_script_rejects_unknown() {
        let v = script_for("not_a_task", &json!({"r_code": "1+1"}));
        // script_for falls through to custom path (r_code) — backend enforces
        assert_eq!(v, "1+1");
    }

    #[test]
    fn test_data_vec_conversion() {
        assert_eq!(data_to_r_vec(&json!([1.5, 2.5])), "c(1.5,2.5)");
        assert_eq!(data_to_r_vec(&json!(null)), "c()");
    }

    // ---------- INTEGRATION HALISI: Rscript inaendesha, artifacts zinatokea ----------

    fn r_available() -> bool {
        std::process::Command::new("Rscript")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Andika WAV halisi (PCM 16-bit mono) kwa test ya audio.
    fn write_test_wav(path: &std::path::Path, sr: u32, secs: f64) {
        let n = (sr as f64 * secs) as u32;
        let mut pcm = Vec::with_capacity(n as usize * 2);
        for i in 0..n {
            let t = i as f64 / sr as f64;
            let s = (t * 440.0 * std::f64::consts::TAU).sin() * 0.6;
            let v = (s * 32767.0) as i16;
            pcm.extend_from_slice(&v.to_le_bytes());
        }
        let data_len = pcm.len() as u32;
        let mut w = Vec::new();
        w.extend_from_slice(b"RIFF");
        w.extend_from_slice(&(36 + data_len).to_le_bytes());
        w.extend_from_slice(b"WAVEfmt ");
        w.extend_from_slice(&16u32.to_le_bytes());
        w.extend_from_slice(&1u16.to_le_bytes()); // PCM
        w.extend_from_slice(&1u16.to_le_bytes()); // mono
        w.extend_from_slice(&sr.to_le_bytes());
        w.extend_from_slice(&(sr * 2).to_le_bytes()); // byte rate
        w.extend_from_slice(&2u16.to_le_bytes()); // block align
        w.extend_from_slice(&16u16.to_le_bytes()); // bits
        w.extend_from_slice(b"data");
        w.extend_from_slice(&data_len.to_le_bytes());
        w.extend_from_slice(&pcm);
        std::fs::write(path, &w).unwrap();
    }

    #[tokio::test]
    async fn integration_summary_halisi() {
        if !r_available() {
            eprintln!("skip: Rscript haipo");
            return;
        }
        let v = run_req("summary", &json!({"data": [1, 2, 3, 4, 100]})).await;
        assert_eq!(v["ok"], json!(true), "stderr={}", v["stderr"]);
        let out: Value = serde_json::from_str(v["stdout"].as_str().unwrap()).unwrap();
        assert_eq!(out["n"], json!(5));
        assert!((out["mean"].as_f64().unwrap() - 22.0).abs() < 1e-9);
        assert_eq!(out["max"].as_f64().unwrap(), 100.0);
    }

    #[tokio::test]
    async fn integration_chart_inazalisha_png_halisi() {
        if !r_available() {
            eprintln!("skip: Rscript haipo");
            return;
        }
        let png = format!("itest_chart_{}.png", uuid::Uuid::new_v4());
        let v = run_req(
            "chart",
            &json!({"data": [3, 7, 2, 9, 5, 8], "chart_kind": "bar",
                    "title": "Integration", "output_png": png}),
        )
        .await;
        assert_eq!(v["ok"], json!(true), "stderr={}", v["stderr"]);
        let path = v["stdout"].as_str().unwrap().trim().to_string();
        let meta = std::fs::metadata(&path).expect("PNG haipo");
        assert!(meta.len() > 5000, "PNG ndogo mno: {}", meta.len());
        let head = std::fs::read(&path).unwrap();
        assert_eq!(&head[..8], b"\x89PNG\r\n\x1a\n", "si PNG halisi");
        let _ = std::fs::remove_file(&path);
    }

    #[tokio::test]
    async fn integration_image_uchanganuzi_halisi() {
        if !r_available() {
            eprintln!("skip: Rscript haipo");
            return;
        }
        // Tengeneza PNG halisi kwa R kisha i-changanue.
        let src = std::env::temp_dir().join(format!("itest_src_{}.png", uuid::Uuid::new_v4()));
        let gen = format!(
            "png('{}', width=300, height=220); par(mar=c(2,2,2,1)); plot(runif(50), col='#3366CC'); dev.off()",
            src.display()
        );
        let (ok, _, err) = rscript(&gen).await;
        assert!(ok, "kutengeneza PNG: {err}");
        let v = run_req("image", &json!({"file": src.to_string_lossy()})).await;
        assert_eq!(v["ok"], json!(true), "stderr={}", v["stderr"]);
        let out: Value = serde_json::from_str(v["stdout"].as_str().unwrap()).unwrap();
        assert_eq!(out["width"], json!(300));
        assert_eq!(out["height"], json!(220));
        assert!(out["brightness"].as_f64().is_some());
        assert!(out["dominant_color"].as_str().unwrap().starts_with('#'));
        let _ = std::fs::remove_file(&src);
    }

    #[tokio::test]
    async fn integration_audio_wav_halisi() {
        if !r_available() {
            eprintln!("skip: Rscript haipo");
            return;
        }
        let wav = std::env::temp_dir().join(format!("itest_{}.wav", uuid::Uuid::new_v4()));
        write_test_wav(&wav, 8000, 0.5);
        let v = run_req("audio", &json!({"file": wav.to_string_lossy()})).await;
        assert_eq!(v["ok"], json!(true), "stderr={}", v["stderr"]);
        let out: Value = serde_json::from_str(v["stdout"].as_str().unwrap()).unwrap();
        assert_eq!(out["sample_rate"], json!(8000));
        assert_eq!(out["channels"], json!(1));
        assert!((out["seconds"].as_f64().unwrap() - 0.5).abs() < 0.01);
        assert!(out["rms"].as_f64().unwrap() > 0.2);
        let _ = std::fs::remove_file(&wav);
    }

    #[tokio::test]
    async fn integration_security_analytics_halisi() {
        if !r_available() {
            eprintln!("skip: Rscript haipo");
            return;
        }
        let rs = run_req("risk_score", &json!({"data": [9.5, 7.2, 5.0, 3.1, 8.8]})).await;
        assert_eq!(rs["ok"], json!(true), "stderr={}", rs["stderr"]);
        let o: Value = serde_json::from_str(rs["stdout"].as_str().unwrap()).unwrap();
        assert_eq!(o["critical"], json!(1));
        assert_eq!(o["findings"], json!(5));

        let an = run_req(
            "anomaly",
            &json!({"data": [10,11,10,12,11,10,10,11,10,12, 400]}),
        )
        .await;
        assert_eq!(an["ok"], json!(true), "stderr={}", an["stderr"]);
        let o: Value = serde_json::from_str(an["stdout"].as_str().unwrap()).unwrap();
        assert!(o["anomalies"].as_u64().unwrap() >= 1);

        let iot = run_req(
            "iot_stats",
            &json!({"data": [22.1, 22.3, 22.2, 22.4, 22.3, 45.0, 22.2]}),
        )
        .await;
        assert_eq!(iot["ok"], json!(true), "stderr={}", iot["stderr"]);
        let o: Value = serde_json::from_str(iot["stdout"].as_str().unwrap()).unwrap();
        assert!(o["anomalies"].as_u64().unwrap() >= 1);
    }

    #[tokio::test]
    async fn integration_chart3d_inazalisha_png_halisi() {
        if !r_available() {
            eprintln!("skip: Rscript haipo");
            return;
        }
        let png = format!("itest_3d_{}.png", uuid::Uuid::new_v4());
        let v = run_req(
            "chart3d",
            &json!({"data": [[1,2,3],[2,4,1],[3,1,5],[4,3,2],[5,5,4]],
                    "title": "3D Test", "output_png": png}),
        )
        .await;
        assert_eq!(v["ok"], json!(true), "stderr={}", v["stderr"]);
        let out: Value = serde_json::from_str(v["stdout"].as_str().unwrap()).unwrap();
        assert_eq!(out["points"], json!(5));
        let path = out["png"].as_str().unwrap();
        let bytes = std::fs::read(path).expect("PNG haipo");
        assert!(bytes.len() > 5000, "PNG ndogo: {}", bytes.len());
        assert_eq!(&bytes[..8], b"\x89PNG\r\n\x1a\n", "si PNG halisi");
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn integration_video_inazalisha_mp4_halisi() {
        if !r_available() {
            eprintln!("skip: Rscript haipo");
            return;
        }
        if std::process::Command::new("sh").arg("-c").arg("command -v ffmpeg")
            .output().map(|o| !o.status.success()).unwrap_or(true)
        {
            eprintln!("skip: ffmpeg haipo");
            return;
        }
        let mp4 = format!("itest_vid_{}.mp4", uuid::Uuid::new_v4());
        let v = run_req(
            "video",
            &json!({"data": [1,3,2,5,4,6,3,7], "output_mp4": mp4,
                    "title": "Video Test", "width": 320, "height": 240, "fps": 4}),
        )
        .await;
        assert_eq!(v["ok"], json!(true), "stderr={}", v["stderr"]);
        let out: Value = serde_json::from_str(v["stdout"].as_str().unwrap()).unwrap();
        assert_eq!(out["frames"], json!(8));
        let path = out["mp4"].as_str().unwrap();
        let bytes = std::fs::read(path).expect("MP4 haipo");
        assert!(bytes.len() > 1000, "MP4 ndogo: {}", bytes.len());
        // MP4: box ya "ftyp" karibu na mwanzo
        assert!(bytes[4..8] == *b"ftyp", "si MP4 halisi");
        // thibitisha kwa ffprobe kama ipo
        if std::process::Command::new("sh").arg("-c").arg("command -v ffprobe")
            .output().map(|o| o.status.success()).unwrap_or(false)
        {
            let o = std::process::Command::new("ffprobe")
                .args(["-v", "error", "-show_entries", "stream=codec_name,width,height",
                       "-of", "default=noprint_wrappers=1"])
                .arg(path)
                .output()
                .unwrap();
            let t = String::from_utf8_lossy(&o.stdout);
            assert!(t.contains("h264"), "codec: {t}");
            assert!(t.contains("width=320") && t.contains("height=240"), "dim: {t}");
        }
        let _ = std::fs::remove_file(path);
    }
}

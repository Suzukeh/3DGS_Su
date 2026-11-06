//! 3DGS の PLY / SOG を AviUtl2 用の `.su3ds` に変換する。
//!
//! Python 版 `tools/3dgs_ply_to_su3ds.py` の移植。外部プロセスを使わず、
//! プラグイン内で完結させる。数式・量子化・出力レイアウトは Python 版に合わせている。

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::Path;

use image::ImageFormat;
use serde_json::Value;

const MAGIC: &[u8; 8] = b"SU3DGS\0\0";
const VERSION_FAST: u32 = 1;
const VERSION_QUALITY: u32 = 2;
const HEADER_SIZE: usize = 40;
const QUALITY_HEADER_SIZE: usize = 48;
const RECORD_SIZE: usize = 40;
const QUALITY_SH_COEFFICIENTS: usize = 45;
const QUALITY_RECORD_SIZE: usize = 36 + 16 + QUALITY_SH_COEFFICIENTS * 4;

const PALETTE_BITS: u32 = 3;
const PALETTE_STEPS: usize = 1 << PALETTE_BITS;
const PALETTE_COLORS: usize = PALETTE_STEPS * PALETTE_STEPS * PALETTE_STEPS;
const PALETTE_LEVELS: [i32; 8] = [0, 16, 40, 72, 112, 160, 208, 255];
const ALPHA_STEPS: usize = 8;
const SH_C0: f64 = 0.282_094_791_773_878_14;

const NEAREST_LEVEL: [u8; 256] = build_nearest_levels();

const fn build_nearest_levels() -> [u8; 256] {
    let mut table = [0u8; 256];
    let mut v = 0;
    while v < 256 {
        let mut best = 0usize;
        let mut best_diff = u32::MAX;
        let mut i = 0;
        while i < PALETTE_LEVELS.len() {
            let level = PALETTE_LEVELS[i];
            let diff = level.abs_diff(v as i32);
            if diff < best_diff {
                best_diff = diff;
                best = i;
            }
            i += 1;
        }
        table[v] = best as u8;
        v += 1;
    }
    table
}

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn error(message: impl Into<String>) -> Box<dyn std::error::Error + Send + Sync> {
    io::Error::other(message.into()).into()
}

#[derive(Clone, Debug)]
pub struct ConvertOptions {
    pub max: usize,
    pub scale: f64,
    pub fit: f64,
    pub flip_y: bool,
    pub flip_z: bool,
    pub no_center: bool,
    pub no_dither: bool,
    pub preserve_quality: bool,
    pub keep_sh: bool,
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self {
            max: 30_000,
            scale: 1.0,
            fit: 400.0,
            flip_y: false,
            flip_z: false,
            no_center: false,
            no_dither: false,
            preserve_quality: false,
            keep_sh: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConvertInfo {
    pub points_in_source: usize,
    pub points_written: usize,
    pub format: String,
    pub sh_bands: Option<u32>,
    pub load_ms: u128,
    pub process_ms: u128,
    pub elapsed_ms: u128,
}

struct RawCloud {
    pos: Vec<[f64; 3]>,
    rgb: Vec<[f64; 3]>,
    alpha: Vec<f64>,
    sh: Vec<[f64; QUALITY_SH_COEFFICIENTS]>,
    log_scale: Vec<[f64; 3]>,
    quat: Vec<[f64; 4]>,
    format: String,
    points_in_source: usize,
    sh_bands: Option<u32>,
    generator: Option<String>,
    license: Option<String>,
}

impl RawCloud {
    fn with_capacity(count: usize, format: impl Into<String>) -> Self {
        Self {
            pos: Vec::with_capacity(count),
            rgb: Vec::with_capacity(count),
            alpha: Vec::with_capacity(count),
            sh: Vec::with_capacity(count),
            log_scale: Vec::with_capacity(count),
            quat: Vec::with_capacity(count),
            format: format.into(),
            points_in_source: count,
            sh_bands: None,
            generator: None,
            license: None,
        }
    }
}

/// 入力ファイルを読み、出力 `.su3ds` と同名の JSON を書く。
pub fn convert_file(input: &Path, output: &Path, options: &ConvertOptions) -> Result<ConvertInfo> {
    let started = std::time::Instant::now();
    let raw = if is_sog(input)? {
        load_sog(
            input,
            options.preserve_quality && options.keep_sh,
            options.preserve_quality,
        )?
    } else {
        load_ply(input)?
    };
    let load_ms = started.elapsed().as_millis();
    let process_started = std::time::Instant::now();

    let count = raw.pos.len();
    if count == 0 {
        return Err(error("有効な点がありません"));
    }

    let mut pos = raw.pos;
    let mut cov = Vec::with_capacity(count);
    for i in 0..count {
        cov.push(covariance(raw.quat[i], raw.log_scale[i]));
    }

    let mut scale = options.scale;
    let mut fit_scale = 1.0;
    if options.fit > 0.0 {
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for p in &pos {
            for axis in 0..3 {
                min[axis] = min[axis].min(p[axis]);
                max[axis] = max[axis].max(p[axis]);
            }
        }
        let extent =
            (max[0] - min[0]).max(max[1] - min[1]).max(max[2] - min[2]) * options.scale.abs();
        if extent > 1e-9 {
            fit_scale = options.fit / extent;
            scale *= fit_scale;
        }
    }

    let scale2 = scale * scale;
    for p in &mut pos {
        p[0] *= scale;
        p[1] *= scale;
        p[2] *= scale;
        if options.flip_y {
            p[1] = -p[1];
        }
        if options.flip_z {
            p[2] = -p[2];
        }
    }
    for c in &mut cov {
        for value in c.iter_mut() {
            *value *= scale2;
        }
        if options.flip_y {
            c[1] = -c[1];
            c[4] = -c[4];
        }
        if options.flip_z {
            c[2] = -c[2];
            c[4] = -c[4];
        }
    }

    let mut order: Vec<usize> = Vec::with_capacity(count);
    for i in 0..count {
        if !pos[i].iter().all(|v| v.is_finite())
            || !cov[i].iter().all(|v| v.is_finite())
            || !raw.alpha[i].is_finite()
            || raw.alpha[i] < 0.0
            || (!options.preserve_quality && raw.alpha[i] <= 0.004)
        {
            continue;
        }
        order.push(i);
    }
    if order.is_empty() {
        return Err(error("有効な点がありません"));
    }

    let mut center = [0.0; 3];
    if !options.no_center {
        let mut min = [f64::INFINITY; 3];
        let mut max = [f64::NEG_INFINITY; 3];
        for &i in &order {
            for axis in 0..3 {
                min[axis] = min[axis].min(pos[i][axis]);
                max[axis] = max[axis].max(pos[i][axis]);
            }
        }
        for axis in 0..3 {
            center[axis] = (min[axis] + max[axis]) * 0.5;
        }
        for &i in &order {
            for axis in 0..3 {
                pos[i][axis] -= center[axis];
            }
        }
    }

    if !options.preserve_quality && options.max > 0 && order.len() > options.max {
        order.sort_unstable_by(|&a, &b| {
            raw.alpha[b]
                .total_cmp(&raw.alpha[a])
                .then_with(|| a.cmp(&b))
        });
        order.truncate(options.max);
    }

    let written = order.len();
    let mut bounds_min = [f64::INFINITY; 3];
    let mut bounds_max = [f64::NEG_INFINITY; 3];
    for &i in &order {
        for axis in 0..3 {
            bounds_min[axis] = bounds_min[axis].min(pos[i][axis]);
            bounds_max[axis] = bounds_max[axis].max(pos[i][axis]);
        }
    }

    if options.preserve_quality {
        // 品質v2は全フィールドを f32 で書き出す。NaN/Inf や f64→f32 の
        // オーバーフローをそのまま出力しないよう、書き込み前に一括検証する。
        // SH は実際に保存する場合だけ対象にする (keep_sh=false では 0 を保存)。
        let require_f32_finite =
            |point: Option<usize>, label: &str, component: usize, value: f64| -> Result<()> {
                if value.is_finite() && (value as f32).is_finite() {
                    return Ok(());
                }
                let reason = if value.is_finite() {
                    "f32 の範囲を超えています"
                } else {
                    "有限値ではありません"
                };
                let point = match point {
                    Some(point) => format!("点 {point} の"),
                    None => String::new(),
                };
                Err(error(format!(
                    "品質v2で出力する{point}{label}[{component}]が{reason} (値 {value})"
                )))
            };
        for &i in &order {
            for (component, value) in pos[i].iter().enumerate() {
                require_f32_finite(Some(i), "位置", component, *value)?;
            }
            for (component, value) in cov[i].iter().enumerate() {
                require_f32_finite(Some(i), "共分散", component, *value)?;
            }
            for (component, value) in raw.rgb[i].iter().enumerate() {
                require_f32_finite(Some(i), "RGB", component, *value)?;
            }
            require_f32_finite(Some(i), "不透明度", 0, raw.alpha[i])?;
            if options.keep_sh {
                for (component, value) in raw.sh[i].iter().enumerate() {
                    require_f32_finite(Some(i), "SH係数", component, *value)?;
                }
            }
        }
        for (axis, value) in bounds_min.iter().enumerate() {
            require_f32_finite(None, "境界最小値", axis, *value)?;
        }
        for (axis, value) in bounds_max.iter().enumerate() {
            require_f32_finite(None, "境界最大値", axis, *value)?;
        }
    }

    let record_size = if options.preserve_quality {
        QUALITY_RECORD_SIZE
    } else {
        RECORD_SIZE
    };
    let mut body = Vec::with_capacity(written * record_size);
    for &i in &order {
        if options.preserve_quality {
            for value in pos[i] {
                body.extend_from_slice(&(value as f32).to_le_bytes());
            }
            for value in cov[i] {
                body.extend_from_slice(&(value as f32).to_le_bytes());
            }
            for value in raw.rgb[i] {
                body.extend_from_slice(&(value as f32).to_le_bytes());
            }
            body.extend_from_slice(&(raw.alpha[i] as f32).to_le_bytes());
            for value in if options.keep_sh {
                raw.sh[i]
            } else {
                [0.0; QUALITY_SH_COEFFICIENTS]
            } {
                body.extend_from_slice(&(value as f32).to_le_bytes());
            }
            continue;
        }

        let mut rgb = raw.rgb[i];
        if !options.no_dither {
            for (channel, value) in rgb.iter_mut().enumerate() {
                *value += dither(i, channel) / 255.0;
            }
        }
        let color = quantize_color(rgb);
        let alpha = (raw.alpha[i] * (ALPHA_STEPS - 1) as f64)
            .round_ties_even()
            .clamp(0.0, (ALPHA_STEPS - 1) as f64) as u8;

        for value in pos[i] {
            body.extend_from_slice(&(value as f32).to_le_bytes());
        }
        for value in cov[i] {
            body.extend_from_slice(&(value as f32).to_le_bytes());
        }
        body.extend_from_slice(&color.to_le_bytes());
        body.push(alpha);
        body.push(0);
    }

    let mut header = Vec::with_capacity(if options.preserve_quality {
        QUALITY_HEADER_SIZE
    } else {
        HEADER_SIZE
    });
    header.extend_from_slice(MAGIC);
    header.extend_from_slice(
        &(if options.preserve_quality {
            VERSION_QUALITY
        } else {
            VERSION_FAST
        })
        .to_le_bytes(),
    );
    header.extend_from_slice(&(written as u32).to_le_bytes());
    for value in bounds_min {
        header.extend_from_slice(&(value as f32).to_le_bytes());
    }
    for value in bounds_max {
        header.extend_from_slice(&(value as f32).to_le_bytes());
    }
    if options.preserve_quality {
        header.extend_from_slice(&(record_size as u32).to_le_bytes());
        let sh_degree = if options.keep_sh {
            raw.sh_bands.unwrap_or(0) & 0x03
        } else {
            0
        };
        let flags = ((sh_degree as u32) << 8)
            | ((sh_degree > 0) as u32)
            | ((options.flip_y as u32) << 1)
            | ((options.flip_z as u32) << 2);
        header.extend_from_slice(&flags.to_le_bytes());
    }
    debug_assert_eq!(
        header.len(),
        if options.preserve_quality {
            QUALITY_HEADER_SIZE
        } else {
            HEADER_SIZE
        }
    );
    debug_assert_eq!(body.len(), written * record_size);

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut data = header;
    data.extend_from_slice(&body);
    fs::write(output, data)?;

    let meta = serde_json::json!({
        "source": input.to_string_lossy(),
        "output": output.to_string_lossy(),
        "format": raw.format,
        "points_in_source": raw.points_in_source,
        "sh_bands": raw.sh_bands,
        "generator": raw.generator,
        "source_license": raw.license,
        "fit_scale": fit_scale,
        "points_written": written,
        "palette_colors": PALETTE_COLORS,
        "alpha_levels": ALPHA_STEPS,
        "bounds_min": bounds_min,
        "bounds_max": bounds_max,
        "center_offset": center,
        "options": {
            "max": options.max,
            "scale": options.scale,
            "fit": options.fit,
            "flip_y": options.flip_y,
            "flip_z": options.flip_z,
        },
    });
    let json = serde_json::to_string_pretty(&meta)?;
    fs::write(output.with_extension("json"), json)?;

    Ok(ConvertInfo {
        points_in_source: raw.points_in_source,
        points_written: written,
        format: raw.format,
        sh_bands: raw.sh_bands,
        load_ms,
        process_ms: process_started.elapsed().as_millis(),
        elapsed_ms: started.elapsed().as_millis(),
    })
}

fn covariance(quat: [f64; 4], log_scale: [f64; 3]) -> [f64; 6] {
    let norm =
        (quat[0] * quat[0] + quat[1] * quat[1] + quat[2] * quat[2] + quat[3] * quat[3]).sqrt();
    let norm = if norm < 1e-12 { 1.0 } else { norm };
    let w = quat[0] / norm;
    let x = quat[1] / norm;
    let y = quat[2] / norm;
    let z = quat[3] / norm;
    let xx = x * x;
    let yy = y * y;
    let zz = z * z;
    let xy = x * y;
    let xz = x * z;
    let yz = y * z;
    let wx = w * x;
    let wy = w * y;
    let wz = w * z;

    let r00 = 1.0 - 2.0 * (yy + zz);
    let r01 = 2.0 * (xy - wz);
    let r02 = 2.0 * (xz + wy);
    let r10 = 2.0 * (xy + wz);
    let r11 = 1.0 - 2.0 * (xx + zz);
    let r12 = 2.0 * (yz - wx);
    let r20 = 2.0 * (xz - wy);
    let r21 = 2.0 * (yz + wx);
    let r22 = 1.0 - 2.0 * (xx + yy);

    let s0 = log_scale[0].clamp(-30.0, 10.0).exp();
    let s1 = log_scale[1].clamp(-30.0, 10.0).exp();
    let s2 = log_scale[2].clamp(-30.0, 10.0).exp();

    let m00 = r00 * s0;
    let m01 = r01 * s1;
    let m02 = r02 * s2;
    let m10 = r10 * s0;
    let m11 = r11 * s1;
    let m12 = r12 * s2;
    let m20 = r20 * s0;
    let m21 = r21 * s1;
    let m22 = r22 * s2;

    [
        m00 * m00 + m01 * m01 + m02 * m02,
        m00 * m10 + m01 * m11 + m02 * m12,
        m00 * m20 + m01 * m21 + m02 * m22,
        m10 * m10 + m11 * m11 + m12 * m12,
        m10 * m20 + m11 * m21 + m12 * m22,
        m20 * m20 + m21 * m21 + m22 * m22,
    ]
}

fn quantize_color(rgb: [f64; 3]) -> u16 {
    let mut result = 0u16;
    for value in rgb {
        let q = (value.clamp(0.0, 1.0) * 255.0)
            .round_ties_even()
            .clamp(0.0, 255.0) as usize;
        result = (result << PALETTE_BITS) | NEAREST_LEVEL[q] as u16;
    }
    result
}

fn dither(index: usize, channel: usize) -> f64 {
    // Python の default_rng とは列が違うが、3bit 量子化のバンディングを
    // 点の重なりで平均化する目的は同じ。決定的で高速なハッシュを使う。
    let mut h = (index as u64)
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add((channel as u64).wrapping_mul(0xD1B5_4A32_D192_ED03));
    h ^= h >> 30;
    h = h.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    h ^= h >> 27;
    h = h.wrapping_mul(0x94D0_49BB_1331_11EB);
    h ^= h >> 31;
    ((h >> 11) as f64 / (1u64 << 53) as f64) * 16.0 - 8.0
}

// ---------------------------------------------------------------------------
// PLY
// ---------------------------------------------------------------------------

fn ply_sh_degree(coefficient_count: usize) -> Result<Option<u32>> {
    match coefficient_count {
        0 => Ok(None),
        9 => Ok(Some(1)),
        24 => Ok(Some(2)),
        45 => Ok(Some(3)),
        count => Err(error(format!(
            "PLY f_rest_* の係数数が不正です ({count}; 9/24/45 が必要です)"
        ))),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PropType {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    F32,
    F64,
}

impl PropType {
    fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "char" | "int8" => Self::I8,
            "uchar" | "uint8" => Self::U8,
            "short" | "int16" => Self::I16,
            "ushort" | "uint16" => Self::U16,
            "int" | "int32" => Self::I32,
            "uint" | "uint32" => Self::U32,
            "float" | "float32" => Self::F32,
            "double" | "float64" => Self::F64,
            _ => return None,
        })
    }

    fn size(self) -> usize {
        match self {
            Self::I8 | Self::U8 => 1,
            Self::I16 | Self::U16 => 2,
            Self::I32 | Self::U32 | Self::F32 => 4,
            Self::F64 => 8,
        }
    }
}

struct PlyHeader {
    format: String,
    count: usize,
    header_len: usize,
    stride: usize,
    props: Vec<(String, PropType, usize)>,
}

fn read_ply_header(data: &[u8]) -> Result<PlyHeader> {
    let marker = b"end_header";
    let pos = data
        .windows(marker.len())
        .position(|window| window == marker)
        .ok_or_else(|| error("PLY の end_header が見つかりません"))?;
    let mut end = pos + marker.len();
    while end < data.len() && data[end] != b'\n' {
        end += 1;
    }
    if end < data.len() {
        end += 1;
    }
    let header = String::from_utf8_lossy(&data[..end]);
    let mut lines = header.lines();
    if lines.next().map(str::trim) != Some("ply") {
        return Err(error("PLY ファイルではありません"));
    }

    let mut format = String::new();
    let mut count = 0usize;
    let mut props: Vec<(String, PropType, usize)> = Vec::new();
    let mut stride = 0usize;
    let mut in_vertex = false;

    for line in lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }
        match parts[0] {
            "format" if parts.len() >= 2 => format = parts[1].to_string(),
            "element" if parts.len() >= 3 => {
                in_vertex = parts[1] == "vertex";
                if in_vertex {
                    count = parts[2]
                        .parse()
                        .map_err(|_| error("PLY の点数が不正です"))?;
                }
            }
            "property" if in_vertex => {
                if parts.len() < 3 {
                    return Err(error("vertex 要素に list プロパティは未対応です"));
                }
                let ty = PropType::from_name(parts[1])
                    .ok_or_else(|| error(format!("未対応の PLY 型: {}", parts[1])))?;
                props.push((parts[2].to_string(), ty, stride));
                stride += ty.size();
            }
            _ => {}
        }
    }

    Ok(PlyHeader {
        format,
        count,
        header_len: end,
        stride,
        props,
    })
}

fn load_ply(path: &Path) -> Result<RawCloud> {
    let data = fs::read(path)?;
    let header = read_ply_header(&data)?;
    if header.count == 0 {
        return Err(error("PLY に vertex がありません"));
    }
    if header.props.is_empty() {
        return Err(error("PLY に vertex プロパティがありません"));
    }

    let mut raw = RawCloud::with_capacity(header.count, header.format.clone());
    if header.format == "ascii" {
        load_ply_ascii(&data[header.header_len..], &header, &mut raw)?;
    } else {
        let big_endian = match header.format.as_str() {
            "binary_little_endian" => false,
            "binary_big_endian" => true,
            other => return Err(error(format!("未対応の PLY format: {other}"))),
        };
        let needed = header
            .header_len
            .checked_add(header.stride.saturating_mul(header.count))
            .ok_or_else(|| error("PLY のサイズが大きすぎます"))?;
        if data.len() < needed {
            return Err(error(format!(
                "PLY のデータが不足しています (必要 {needed} / 実際 {} バイト)",
                data.len()
            )));
        }
        load_ply_binary(&data, &header, big_endian, &mut raw)?;
    }
    raw.points_in_source = header.count;
    Ok(raw)
}

fn property_offset(header: &PlyHeader, names: &[&str]) -> Result<(usize, PropType)> {
    for (name, ty, offset) in &header.props {
        if names.contains(&name.as_str()) {
            return Ok((*offset, *ty));
        }
    }
    Err(error(format!(
        "PLY に必要なプロパティがありません: {}",
        names.join(", ")
    )))
}

fn load_ply_binary(
    data: &[u8],
    header: &PlyHeader,
    big_endian: bool,
    raw: &mut RawCloud,
) -> Result<()> {
    let (x_off, x_ty) = property_offset(header, &["x"])?;
    let (y_off, y_ty) = property_offset(header, &["y"])?;
    let (z_off, z_ty) = property_offset(header, &["z"])?;
    let (op_off, op_ty) = property_offset(header, &["opacity", "alpha"])?;
    let (s0_off, s0_ty) = property_offset(header, &["scale_0"])?;
    let (s1_off, s1_ty) = property_offset(header, &["scale_1"])?;
    let (s2_off, s2_ty) = property_offset(header, &["scale_2"])?;
    let (r0_off, r0_ty) = property_offset(header, &["rot_0"])?;
    let (r1_off, r1_ty) = property_offset(header, &["rot_1"])?;
    let (r2_off, r2_ty) = property_offset(header, &["rot_2"])?;
    let (r3_off, r3_ty) = property_offset(header, &["rot_3"])?;

    let color = if header.props.iter().any(|(name, _, _)| name == "f_dc_0") {
        let (a, at) = property_offset(header, &["f_dc_0"])?;
        let (b, bt) = property_offset(header, &["f_dc_1"])?;
        let (c, ct) = property_offset(header, &["f_dc_2"])?;
        Some((a, at, b, bt, c, ct, true))
    } else {
        let (a, at) = property_offset(header, &["red"])?;
        let (b, bt) = property_offset(header, &["green"])?;
        let (c, ct) = property_offset(header, &["blue"])?;
        Some((a, at, b, bt, c, ct, false))
    };
    let (cr, crt, cg, cgt, cb, cbt, is_dc) = color.ok_or_else(|| error("PLY に色がありません"))?;
    let sh_props: Vec<Option<(usize, PropType)>> = (0..QUALITY_SH_COEFFICIENTS)
        .map(|i| property_offset(header, &[&format!("f_rest_{i}")]).ok())
        .collect();
    let sh_count = sh_props.iter().take_while(|prop| prop.is_some()).count();
    if sh_props.iter().skip(sh_count).any(Option::is_some) {
        return Err(error("PLY の f_rest_* が連続していません"));
    }
    let sh_degree = ply_sh_degree(sh_count)?;
    raw.sh_bands = sh_degree;

    let mut base = header.header_len;
    for _ in 0..header.count {
        let x = read_prop(data, base + x_off, x_ty, big_endian);
        let y = read_prop(data, base + y_off, y_ty, big_endian);
        let z = read_prop(data, base + z_off, z_ty, big_endian);
        let op = read_prop(data, base + op_off, op_ty, big_endian);
        let s0 = read_prop(data, base + s0_off, s0_ty, big_endian);
        let s1 = read_prop(data, base + s1_off, s1_ty, big_endian);
        let s2 = read_prop(data, base + s2_off, s2_ty, big_endian);
        let r0 = read_prop(data, base + r0_off, r0_ty, big_endian);
        let r1 = read_prop(data, base + r1_off, r1_ty, big_endian);
        let r2 = read_prop(data, base + r2_off, r2_ty, big_endian);
        let r3 = read_prop(data, base + r3_off, r3_ty, big_endian);
        let c0 = read_prop(data, base + cr, crt, big_endian);
        let c1 = read_prop(data, base + cg, cgt, big_endian);
        let c2 = read_prop(data, base + cb, cbt, big_endian);

        raw.pos.push([x, y, z]);
        raw.log_scale.push([s0, s1, s2]);
        raw.quat.push([r0, r1, r2, r3]);
        raw.alpha.push(sigmoid(op));
        if is_dc {
            raw.rgb
                .push([0.5 + SH_C0 * c0, 0.5 + SH_C0 * c1, 0.5 + SH_C0 * c2]);
        } else {
            let max = c0.max(c1).max(c2);
            let divisor = if max > 1.5 { 255.0 } else { 1.0 };
            raw.rgb.push([
                (c0 / divisor).clamp(0.0, 1.0),
                (c1 / divisor).clamp(0.0, 1.0),
                (c2 / divisor).clamp(0.0, 1.0),
            ]);
        }
        let mut sh = [0.0; QUALITY_SH_COEFFICIENTS];
        let coeffs_per_channel = sh_count / 3;
        for (source_index, prop) in sh_props.iter().enumerate().take(sh_count) {
            let target_index =
                (source_index / coeffs_per_channel) * 15 + source_index % coeffs_per_channel;
            let (offset, ty) = prop.expect("validated SH property range");
            sh[target_index] = read_prop(data, base + offset, ty, big_endian);
        }
        raw.sh.push(sh);
        base += header.stride;
    }
    Ok(())
}

fn load_ply_ascii(data: &[u8], header: &PlyHeader, raw: &mut RawCloud) -> Result<()> {
    let names: Vec<&str> = header
        .props
        .iter()
        .map(|(name, _, _)| name.as_str())
        .collect();
    let index = |target: &str| {
        names
            .iter()
            .position(|name| *name == target)
            .ok_or_else(|| error(format!("PLY に必要なプロパティがありません: {target}")))
    };
    let x = index("x")?;
    let y = index("y")?;
    let z = index("z")?;
    let op = index("opacity").or_else(|_| index("alpha"))?;
    let s0 = index("scale_0")?;
    let s1 = index("scale_1")?;
    let s2 = index("scale_2")?;
    let r0 = index("rot_0")?;
    let r1 = index("rot_1")?;
    let r2 = index("rot_2")?;
    let r3 = index("rot_3")?;
    let is_dc = names.contains(&"f_dc_0");
    let (cr, cg, cb) = if is_dc {
        (index("f_dc_0")?, index("f_dc_1")?, index("f_dc_2")?)
    } else {
        (index("red")?, index("green")?, index("blue")?)
    };
    let sh_indices: Vec<Option<usize>> = (0..QUALITY_SH_COEFFICIENTS)
        .map(|i| names.iter().position(|name| *name == format!("f_rest_{i}")))
        .collect();
    let sh_count = sh_indices
        .iter()
        .take_while(|index| index.is_some())
        .count();
    if sh_indices.iter().skip(sh_count).any(Option::is_some) {
        return Err(error("PLY の f_rest_* が連続していません"));
    }
    raw.sh_bands = ply_sh_degree(sh_count)?;

    let text = String::from_utf8_lossy(data);
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < names.len() {
            continue;
        }
        let value = |i: usize| -> f64 { parts[i].parse().unwrap_or(f64::NAN) };
        raw.pos.push([value(x), value(y), value(z)]);
        raw.log_scale.push([value(s0), value(s1), value(s2)]);
        raw.quat.push([value(r0), value(r1), value(r2), value(r3)]);
        raw.alpha.push(sigmoid(value(op)));
        if is_dc {
            raw.rgb.push([
                0.5 + SH_C0 * value(cr),
                0.5 + SH_C0 * value(cg),
                0.5 + SH_C0 * value(cb),
            ]);
        } else {
            let c0 = value(cr);
            let c1 = value(cg);
            let c2 = value(cb);
            let divisor = if c0.max(c1).max(c2) > 1.5 { 255.0 } else { 1.0 };
            raw.rgb.push([
                (c0 / divisor).clamp(0.0, 1.0),
                (c1 / divisor).clamp(0.0, 1.0),
                (c2 / divisor).clamp(0.0, 1.0),
            ]);
        }
        let mut sh = [0.0; QUALITY_SH_COEFFICIENTS];
        let coeffs_per_channel = sh_count / 3;
        for (source_index, prop) in sh_indices.iter().enumerate().take(sh_count) {
            let target_index =
                (source_index / coeffs_per_channel) * 15 + source_index % coeffs_per_channel;
            sh[target_index] = value(prop.expect("validated SH property range"));
        }
        raw.sh.push(sh);
    }
    Ok(())
}

#[inline]
fn read_prop(data: &[u8], offset: usize, ty: PropType, big_endian: bool) -> f64 {
    let bytes = |n: usize| &data[offset..offset + n];
    match ty {
        PropType::I8 => bytes(1)[0] as i8 as f64,
        PropType::U8 => bytes(1)[0] as f64,
        PropType::I16 => {
            let b = [data[offset], data[offset + 1]];
            if big_endian {
                i16::from_be_bytes(b) as f64
            } else {
                i16::from_le_bytes(b) as f64
            }
        }
        PropType::U16 => {
            let b = [data[offset], data[offset + 1]];
            if big_endian {
                u16::from_be_bytes(b) as f64
            } else {
                u16::from_le_bytes(b) as f64
            }
        }
        PropType::I32 => read_u32(data, offset, big_endian) as i32 as f64,
        PropType::U32 => read_u32(data, offset, big_endian) as f64,
        PropType::F32 => f32::from_bits(read_u32(data, offset, big_endian)) as f64,
        PropType::F64 => {
            let b: [u8; 8] = bytes(8).try_into().expect("validated property bounds");
            if big_endian {
                f64::from_be_bytes(b)
            } else {
                f64::from_le_bytes(b)
            }
        }
    }
}

#[inline]
fn read_u32(data: &[u8], offset: usize, big_endian: bool) -> u32 {
    let b = [
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ];
    if big_endian {
        u32::from_be_bytes(b)
    } else {
        u32::from_le_bytes(b)
    }
}

#[inline]
fn sigmoid(value: f64) -> f64 {
    1.0 / (1.0 + (-value).exp())
}

// ---------------------------------------------------------------------------
// SOG
// ---------------------------------------------------------------------------

fn is_sog(path: &Path) -> Result<bool> {
    if path.is_dir() {
        return Ok(path.join("meta.json").is_file());
    }
    if path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("sog"))
    {
        return Ok(true);
    }
    let file = File::open(path)?;
    Ok(zip::ZipArchive::new(file).is_ok())
}

fn load_sog(path: &Path, require_sh_decode: bool, preserve_quality: bool) -> Result<RawCloud> {
    type SogReader = Box<dyn Fn(&str) -> Result<Vec<u8>>>;
    let (meta, license, read_file): (Value, Option<String>, SogReader) = if path.is_dir() {
        let meta_bytes = fs::read(path.join("meta.json"))?;
        let meta: Value = serde_json::from_slice(&meta_bytes)?;
        let license = fs::read_to_string(path.join("license.txt")).ok();
        let root = path.to_path_buf();
        let reader = Box::new(move |name: &str| Ok(fs::read(root.join(name))?));
        (meta, license, reader)
    } else {
        let file = File::open(path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        let meta: Value = {
            let mut entry = archive.by_name("meta.json")?;
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            serde_json::from_slice(&bytes)?
        };
        let license = {
            match archive.by_name("license.txt") {
                Ok(mut entry) => {
                    let mut text = String::new();
                    entry.read_to_string(&mut text).ok();
                    Some(text)
                }
                Err(_) => None,
            }
        };
        // zip はファイルごとに開き直す。SOG の数 MB なら十分速い。
        let path = path.to_path_buf();
        let reader = Box::new(move |name: &str| -> Result<Vec<u8>> {
            let file = File::open(&path)?;
            let mut archive = zip::ZipArchive::new(file)?;
            let mut entry = archive.by_name(name)?;
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes)?;
            Ok(bytes)
        });
        (meta, license, reader)
    };

    let count = meta["count"]
        .as_u64()
        .ok_or_else(|| error("SOG の count がありません"))? as usize;
    if count == 0 {
        return Err(error("SOG の点数が 0 です"));
    }

    let means = &meta["means"];
    let mins = read_vec3(&means["mins"])?;
    let maxs = read_vec3(&means["maxs"])?;
    let means_files = read_files(&means["files"])?;
    let scale_files = read_files(&meta["scales"]["files"])?;
    let quat_files = read_files(&meta["quats"]["files"])?;
    let sh0_files = read_files(&meta["sh0"]["files"])?;
    if means_files.len() < 2
        || scale_files.is_empty()
        || quat_files.is_empty()
        || sh0_files.is_empty()
    {
        return Err(error("SOG の画像ファイル指定が不足しています"));
    }

    let means_l = decode_sog_image(&read_file(&means_files[0])?, count)?;
    let means_u = decode_sog_image(&read_file(&means_files[1])?, count)?;
    let scales = decode_sog_image(&read_file(&scale_files[0])?, count)?;
    let quats = decode_sog_image(&read_file(&quat_files[0])?, count)?;
    let sh0 = decode_sog_image(&read_file(&sh0_files[0])?, count)?;

    let scale_cb = read_codebook(&meta["scales"]["codebook"])?;
    let sh_cb = read_codebook(&meta["sh0"]["codebook"])?;
    if scale_cb.len() < 256 || sh_cb.len() < 256 {
        return Err(error("SOG の codebook が 256 要素未満です"));
    }
    let sh_bands = meta["shN"]["bands"].as_u64().map(|v| v as u32);
    let sog_has_shn = meta["shN"]["files"]
        .as_array()
        .is_some_and(|files| !files.is_empty());
    let sh_coeff_count = match sh_bands.unwrap_or(0) {
        0 => 0,
        1 => 3,
        2 => 8,
        3 => 15,
        bands => {
            return Err(error(format!(
                "SOG SH band {bands} はv2形式の45係数を超えるため非対応です"
            )));
        }
    };
    let sh_values = if require_sh_decode && sog_has_shn {
        let files = read_files(&meta["shN"]["files"])?;
        if files.len() < 2 {
            return Err(error("SOG shN の labels/centroids が不足しています"));
        }
        let centroids_file = read_file(&files[0])?;
        let labels_file = read_file(&files[1])?;
        let (centroids, width) = decode_sog_image_with_width(&centroids_file)?;
        let labels = decode_sog_image(&labels_file, count)?;
        let codebook = read_codebook(&meta["shN"]["codebook"])?;
        let version = meta["version"].as_u64().unwrap_or(2);
        let mut values = vec![[0.0; QUALITY_SH_COEFFICIENTS]; count];
        if sh_coeff_count > 0 {
            for i in 0..count {
                let n = labels[i * 4] as usize
                    | ((labels[i * 4 + 1] as usize) << 8)
                    | if version >= 3 {
                        (labels[i * 4 + 2] as usize) << 16
                    } else {
                        0
                    };
                let u = (n % 64) * sh_coeff_count;
                let v = n / 64;
                for channel in 0..3 {
                    for k in 0..sh_coeff_count {
                        let at = ((u + k) * 4 + channel) + v * width * 4;
                        let byte = *centroids
                            .get(at)
                            .ok_or_else(|| error("SOG shN centroid index が画像範囲外です"))?;
                        let decoded = if version >= 2 {
                            *codebook
                                .get(byte as usize)
                                .ok_or_else(|| error("SOG shN codebook index が範囲外です"))?
                        } else {
                            let t = byte as f64 / 255.0;
                            let lo = meta["shN"]["mins"].as_f64().unwrap_or(0.0);
                            let hi = meta["shN"]["maxs"].as_f64().unwrap_or(0.0);
                            lo * (1.0 - t) + hi * t
                        };
                        values[i][channel * 15 + k] = decoded;
                    }
                }
            }
        }
        Some(values)
    } else {
        if require_sh_decode && sh_coeff_count > 0 {
            return Err(error(
                "SOG shN の画像ファイルがありません。--no-sh でSH保持を無効にできます",
            ));
        }
        None
    };
    let generator = meta["asset"]["generator"].as_str().map(ToString::to_string);

    let mut raw = RawCloud::with_capacity(count, "sog");
    raw.sh_bands = sh_bands;
    raw.generator = generator;
    raw.license = license;

    const SQRT2: f64 = std::f64::consts::SQRT_2;
    for i in 0..count {
        let q0 = ((means_u[i * 4] as u32) << 8) | means_l[i * 4] as u32;
        let q1 = ((means_u[i * 4 + 1] as u32) << 8) | means_l[i * 4 + 1] as u32;
        let q2 = ((means_u[i * 4 + 2] as u32) << 8) | means_l[i * 4 + 2] as u32;
        let mut pos = [0.0; 3];
        for (axis, q) in [q0, q1, q2].into_iter().enumerate() {
            let n = mins[axis] + (maxs[axis] - mins[axis]) * (q as f64 / 65535.0);
            pos[axis] = n.signum() * (n.abs().exp() - 1.0);
        }

        let a = (quats[i * 4] as f64 / 255.0 * 2.0 - 1.0) / SQRT2;
        let b = (quats[i * 4 + 1] as f64 / 255.0 * 2.0 - 1.0) / SQRT2;
        let c = (quats[i * 4 + 2] as f64 / 255.0 * 2.0 - 1.0) / SQRT2;
        let mode = quats[i * 4 + 3] as i32 - 252;
        if !(0..=3).contains(&mode) {
            return Err(error("SOG の quaternion mode が不正です"));
        }
        let d = (1.0 - (a * a + b * b + c * c)).max(0.0).sqrt();
        let quat = match mode {
            0 => [d, a, b, c],
            1 => [a, d, b, c],
            2 => [a, b, d, c],
            _ => [a, b, c, d],
        };

        let color = [
            0.5 + sh_cb[sh0[i * 4] as usize] * SH_C0,
            0.5 + sh_cb[sh0[i * 4 + 1] as usize] * SH_C0,
            0.5 + sh_cb[sh0[i * 4 + 2] as usize] * SH_C0,
        ];
        let alpha = if preserve_quality {
            sh0[i * 4 + 3] as f64 / 255.0
        } else {
            (sh0[i * 4 + 3] as f64 / 255.0).clamp(1.0 / 255.0, 254.0 / 255.0)
        };
        let opacity = (alpha / (1.0 - alpha)).ln();

        raw.pos.push(pos);
        raw.rgb.push(color);
        raw.alpha.push(sigmoid(opacity));
        raw.log_scale.push([
            scale_cb[scales[i * 4] as usize],
            scale_cb[scales[i * 4 + 1] as usize],
            scale_cb[scales[i * 4 + 2] as usize],
        ]);
        raw.quat.push(quat);
        raw.sh.push(
            sh_values
                .as_ref()
                .map(|values| values[i])
                .unwrap_or([0.0; QUALITY_SH_COEFFICIENTS]),
        );
    }

    raw.points_in_source = count;
    Ok(raw)
}

fn read_vec3(value: &Value) -> Result<[f64; 3]> {
    let array = value
        .as_array()
        .ok_or_else(|| error("SOG の mins/maxs が不正です"))?;
    if array.len() < 3 {
        return Err(error("SOG の mins/maxs が不足しています"));
    }
    Ok([
        array[0].as_f64().unwrap_or(0.0),
        array[1].as_f64().unwrap_or(0.0),
        array[2].as_f64().unwrap_or(0.0),
    ])
}

fn read_files(value: &Value) -> Result<Vec<String>> {
    let array = value
        .as_array()
        .ok_or_else(|| error("SOG の files が不正です"))?;
    Ok(array
        .iter()
        .filter_map(|v| v.as_str().map(ToString::to_string))
        .collect())
}

fn read_codebook(value: &Value) -> Result<Vec<f64>> {
    let array = value
        .as_array()
        .ok_or_else(|| error("SOG の codebook が不正です"))?;
    Ok(array.iter().map(|v| v.as_f64().unwrap_or(0.0)).collect())
}

fn decode_sog_image_with_width(bytes: &[u8]) -> Result<(Vec<u8>, usize)> {
    let image = image::load_from_memory_with_format(bytes, ImageFormat::WebP)
        .map_err(|e| error(format!("SOG の WebP をデコードできません: {e}")))?;
    let width = image.width() as usize;
    let rgba = image.to_rgba8().into_raw();
    Ok((rgba, width))
}

fn decode_sog_image(bytes: &[u8], count: usize) -> Result<Vec<u8>> {
    let image = image::load_from_memory_with_format(bytes, ImageFormat::WebP)
        .map_err(|e| error(format!("SOG の WebP をデコードできません: {e}")))?
        .to_rgba8();
    let raw = image.into_raw();
    if raw.len() / 4 < count {
        return Err(error(format!(
            "SOG の画像サイズが不足しています (必要 {count} / 実際 {})",
            raw.len() / 4
        )));
    }
    Ok(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ascii_ply(points: usize) -> Vec<u8> {
        let mut header = format!("ply\nformat ascii 1.0\nelement vertex {points}\n");
        for name in [
            "x", "y", "z", "f_dc_0", "f_dc_1", "f_dc_2", "opacity", "scale_0", "scale_1",
            "scale_2", "rot_0", "rot_1", "rot_2", "rot_3",
        ] {
            header.push_str(&format!("property float {name}\n"));
        }
        for i in 0..QUALITY_SH_COEFFICIENTS {
            header.push_str(&format!("property float f_rest_{i}\n"));
        }
        header.push_str("end_header\n");
        for i in 0..points {
            let mut values = vec![
                i as f32, 0.0, 0.0, 0.2, 0.3, 0.4, -8.0, -2.0, -2.0, -2.0, 1.0, 0.0, 0.0, 0.0,
            ];
            values.extend((0..QUALITY_SH_COEFFICIENTS).map(|j| if j == 0 { 0.125 } else { 0.0 }));
            header.push_str(
                &values
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" "),
            );
            header.push('\n');
        }
        header.into_bytes()
    }

    fn with_fixture<T>(data: &[u8], run: impl FnOnce(&Path, &Path) -> T) -> T {
        static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("su3ds-test-{}-{id}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let input = root.join("fixture.ply");
        let output = root.join("fixture.su3ds");
        fs::write(&input, data).unwrap();
        let result = run(&input, &output);
        let _ = fs::remove_dir_all(root);
        result
    }

    #[test]
    fn double_precision_properties_decode_in_both_byte_orders() {
        let value = 0.125_f64;
        assert_eq!(
            read_prop(&value.to_le_bytes(), 0, PropType::F64, false),
            value
        );
        assert_eq!(
            read_prop(&value.to_be_bytes(), 0, PropType::F64, true),
            value
        );
    }

    #[test]
    fn quality_keeps_every_point_and_float_color_alpha_and_sh() {
        with_fixture(&ascii_ply(2), |input, output| {
            let options = ConvertOptions {
                preserve_quality: true,
                max: 1,
                no_dither: false,
                ..Default::default()
            };
            let info = convert_file(input, output, &options).unwrap();
            assert_eq!(info.points_written, 2);
            let bytes = fs::read(output).unwrap();
            assert_eq!(
                u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
                VERSION_QUALITY
            );
            assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), 2);
            assert_eq!(
                u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize,
                QUALITY_RECORD_SIZE
            );
            let r = &bytes[QUALITY_HEADER_SIZE..QUALITY_HEADER_SIZE + QUALITY_RECORD_SIZE];
            let floats: Vec<f32> = r
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
                .collect();
            assert!((floats[9] - (0.5 + SH_C0 as f32 * 0.2)).abs() < 1e-6);
            assert!((floats[12] - sigmoid(-8.0) as f32).abs() < 1e-6);
            assert_eq!(floats[13], 0.125);
        });
    }

    #[test]
    fn default_conversion_keeps_v1_record_layout() {
        let fixture = String::from_utf8(ascii_ply(1))
            .unwrap()
            .replace(" -8 ", " -2 ");
        with_fixture(fixture.as_bytes(), |input, output| {
            convert_file(input, output, &ConvertOptions::default()).unwrap();
            let bytes = fs::read(output).unwrap();
            assert_eq!(
                u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
                VERSION_FAST
            );
            assert_eq!(bytes.len(), HEADER_SIZE + RECORD_SIZE);
        });
    }

    fn quality_error_for(fixture: &[u8]) -> String {
        with_fixture(fixture, |input, output| {
            let options = ConvertOptions {
                preserve_quality: true,
                ..Default::default()
            };
            let err = convert_file(input, output, &options).unwrap_err();
            assert!(!output.exists(), "不正入力なのに出力が作成されました");
            err.to_string()
        })
    }

    #[test]
    fn quality_rejects_non_finite_rgb_and_sh() {
        let base = || String::from_utf8(ascii_ply(1)).unwrap();
        let cases = [
            (
                base()
                    .replace(" 0.2 0.3 0.4 ", " nan 0.3 0.4 ")
                    .into_bytes(),
                "RGB",
            ),
            (
                base()
                    .replace(" 0.2 0.3 0.4 ", " inf 0.3 0.4 ")
                    .into_bytes(),
                "RGB",
            ),
            (base().replace(" 0.125 0", " inf 0").into_bytes(), "SH係数"),
        ];
        for (fixture, expected) in cases {
            let message = quality_error_for(&fixture);
            assert!(
                message.contains(expected),
                "{expected} のエラーではありません: {message}"
            );
            assert!(
                message.contains("品質v2"),
                "日本語の品質v2エラーではありません: {message}"
            );
        }
    }

    #[test]
    fn quality_rejects_values_that_overflow_f32() {
        let base = || String::from_utf8(ascii_ply(1)).unwrap();
        let fixtures = [
            base()
                .replace(" 0.2 0.3 0.4 ", " 1e300 0.3 0.4 ")
                .into_bytes(),
            base().replace(" 0.125 0", " 1e300 0").into_bytes(),
        ];
        for fixture in fixtures {
            let message = quality_error_for(&fixture);
            assert!(
                message.contains("f32"),
                "f32 範囲超過のエラーではありません: {message}"
            );
        }
    }

    #[test]
    fn quality_does_not_validate_sh_when_it_is_not_saved() {
        let fixture = String::from_utf8(ascii_ply(1))
            .unwrap()
            .replace(" 0.125 0", " nan 0");
        with_fixture(fixture.as_bytes(), |input, output| {
            let options = ConvertOptions {
                preserve_quality: true,
                keep_sh: false,
                ..Default::default()
            };
            let info = convert_file(input, output, &options).unwrap();
            assert_eq!(info.points_written, 1);
            let bytes = fs::read(output).unwrap();
            let record = &bytes[QUALITY_HEADER_SIZE..QUALITY_HEADER_SIZE + QUALITY_RECORD_SIZE];
            for chunk in record[36 + 16..].chunks_exact(4) {
                assert_eq!(f32::from_le_bytes(chunk.try_into().unwrap()), 0.0);
            }
        });
    }
}

use hound::{WavReader, WavWriter, WavSpec};

// --- НАШИ СТРУКТУРЫ БИКВАДРАТНОГО ФИЛЬТРА ---

#[derive(Debug, Clone, Copy)]
pub struct BiquadCoefficients {
    b0: f32, b1: f32, b2: f32,
    a1: f32, a2: f32,
}

impl BiquadCoefficients {
    pub fn low_pass(cutoff_hz: f32, sample_rate: f32, q: f32) -> Self {
        let cutoff = cutoff_hz.clamp(20.0, sample_rate * 0.49);
        let omega = 2.0 * std::f32::consts::PI * cutoff / sample_rate;
        let cos_w = omega.cos();
        let sin_w = omega.sin();
        let alpha = sin_w / (2.0 * q);
        let a0 = 1.0 + alpha;

        Self {
            b0: ((1.0 - cos_w) / 2.0) / a0,
            b1: (1.0 - cos_w) / a0,
            b2: ((1.0 - cos_w) / 2.0) / a0,
            a1: (-2.0 * cos_w) / a0,
            a2: (1.0 - alpha) / a0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BiquadState {
    s1: f32,
    s2: f32,
}

impl BiquadState {
    #[inline(always)]
    pub fn process_sample(&mut self, input: f32, coeffs: &BiquadCoefficients) -> f32 {
        let output = coeffs.b0 * input + self.s1;
        self.s1 = coeffs.b1 * input - coeffs.a1 * output + self.s2;
        self.s2 = coeffs.b2 * input - coeffs.a2 * output;
        output
    }
}

// --- ОСНОВНАЯ ЛОГИКА ---

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Входной и выходной файлы (положите тестовый input.wav в корень проекта)
    let input_path = "input.wav";
    let output_path = "output_filtered.wav";

    println!("Открываем файл: {}", input_path);
    let mut reader = WavReader::open(input_path)?;
    let spec = reader.spec();

    println!("Параметры аудио:");
    println!("  Частота дискретизации: {} Гц", spec.sample_rate);
    println!("  Каналы: {}", spec.channels);
    println!("  Бит на сэмпл: {}", spec.bits_per_sample);

    // Поддерживаем только 16-битный PCM для простоты примера
    if spec.bits_per_sample != 16 {
        panic!("Поддерживается только 16-битный WAV формат.");
    }

    // Создаем коэффициенты фильтра.
    // Срез на 800 Гц (все высокие частоты будут сильно приглушены, как за стеной)
    let cutoff_hz = 800.0;
    let coeffs = BiquadCoefficients::low_pass(
        cutoff_hz,
        spec.sample_rate as f32,
        std::f32::consts::FRAC_1_SQRT_2 // Добротность Баттерворта
    );

    // Создаем массив состояний фильтра под количество каналов
    let mut channel_states = vec![BiquadState::default(); spec.channels as usize];

    // Открываем файл для записи результата
    let mut writer = WavWriter::create(output_path, spec)?;

    println!("Начало фильтрации...");

    // Читаем сэмплы, фильтруем и записываем
    // Перечисляем индекс i для определения текущего канала
    for (i, sample_result) in reader.samples::<i16>().enumerate() {
        let raw_sample = sample_result?;

        // 1. Нормализация: i16 -> f32 в диапазоне [-1.0, 1.0]
        let float_sample = raw_sample as f32 / 32768.0;

        // Определяем индекс канала для текущего сэмпла
        let channel = i % (spec.channels as usize);

        // 2. Обработка фильтром
        let filtered_float = channel_states[channel].process_sample(float_sample, &coeffs);

        // 3. Денормализация: f32 -> i16 с ограничением (clamping) во избежание переполнения
        let filtered_int = (filtered_float * 32767.0).clamp(-32768.0, 32767.0) as i16;

        writer.write_sample(filtered_int)?;
    }

    writer.finalize()?;
    println!("Готово! Отфильтрованный файл сохранен в: {}", output_path);

    Ok(())
}
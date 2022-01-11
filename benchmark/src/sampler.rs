use std::collections::HashMap;
use anyhow::anyhow;
use hdrhistogram::Histogram as HdrHistogram;
use tokio::sync::oneshot;
use tokio::time::Duration;
use plotters::prelude::*;

pub(crate) type ChannelMessage = SampleData;

/// The data sampled from the benchmark
pub(crate) struct SampleData {
    /// All request latencies.
    latencies: Vec<Duration>,

    /// All request latencies.
    sentence_length_latencies: Vec<Vec<Duration>>,

    errors: HashMap<u16, usize>,

    requests_second: f64,
}

pub(crate) struct SamplerHandle {
    sample: SampleData,
    submit: oneshot::Sender<ChannelMessage>,
}

impl SamplerHandle {
    pub(crate) fn finish(mut self) {
        let total_elapsed = self.sample.latencies.iter().sum::<Duration>();
        self.sample.requests_second = self.sample.latencies.len() as f64 / total_elapsed.as_secs_f64();
        dbg!(total_elapsed, self.sample.requests_second);

        let _ = self.submit.send(self.sample);
    }

    pub(crate) fn new() -> (Self, oneshot::Receiver<ChannelMessage>) {
        let sample = SampleData {
            latencies: vec![],
            sentence_length_latencies: vec![],
            errors: HashMap::new(),
            requests_second: 0.0
        };

        let (tx, rx) = oneshot::channel();

        let inst = Self {
            sample,
            submit: tx,
        };

        (inst, rx)
    }

    pub(crate) fn add_latency(&mut self, dur: Duration) {
        self.sample.latencies.push(dur);
    }

    pub(crate) fn add_latency_for_sentence_length(&mut self, length: usize, dur: Duration) {
        if length >= self.sample.sentence_length_latencies.len() {
            for i in self.sample.sentence_length_latencies.len()..length + 1 {
                self.sample.sentence_length_latencies.insert(i, vec![]);
            }
        }

        self.sample.sentence_length_latencies[length].push(dur);
    }

    pub(crate) fn register_error(&mut self, status: u16) {
        let exists = self.sample.errors.get(&status);
        let v = if let Some(v) = exists { *v + 1 } else { 1 };
        self.sample.errors.insert(status, v);
    }
}

pub(crate) struct Sampler {
    output: String,
    sample_handles: Vec<oneshot::Receiver<ChannelMessage>>,
}

impl Sampler {
    pub(crate) fn new(output: String) -> Self {
        Self {
            output,
            sample_handles: vec![],
        }
    }

    pub(crate) fn get_handle(&mut self) -> SamplerHandle {
        let (handler, rx) = SamplerHandle::new();

        self.sample_handles.push(rx);

        handler
    }

    pub(crate) async fn wait_and_sample(self) -> anyhow::Result<()> {
        let mut req_sec = vec![];
        let mut all_results: Vec<Duration> = vec![];
        let mut all_sentence_length_latencies: HashMap<usize, Vec<Duration>> = HashMap::new();
        let mut errors = HashMap::new();
        let output = format!("{}/run-output.png", self.output);

        for sample in self.sample_handles {
            let mut res = match sample.await {
                Ok(r) => r,
                Err(_) => continue,
            };

            req_sec.push(res.requests_second);
            all_results.append(&mut res.latencies);

            for (length, mut latencies) in res.sentence_length_latencies.drain(..).enumerate() {
                let contains = {
                    all_sentence_length_latencies.contains_key(&length)
                };

                if contains {
                    all_sentence_length_latencies
                        .get_mut(&length)
                        .unwrap()
                        .append(&mut latencies);
                } else {
                    all_sentence_length_latencies.insert(length, latencies);
                }
            }

            for (status, count) in res.errors {
                let v = errors.get(&status);
                let v = if let Some(v) = v { *v + count } else { count };

                errors.insert(status, v);
            }
        }

        if all_results.is_empty() {
            return Err(anyhow!("Unable to succesfully complete test due to no tasks succeeding"));
        }

        let mut hist = HdrHistogram::<u64>::new_with_bounds(1, 60 * 60 * 1000, 2).unwrap();

        hist.auto(true);
        for result in all_results.iter() {
            if result.as_micros() > 0 {
                hist.record(result.as_micros() as u64)?;
            }
        }

        // Calculate the total time spent handling successful requests by adding up all the time
        // taken processing the requests then divide by the concurrency factor as that allows upto
        // n requests to happen in parallel.
        let requests_a_sec = req_sec.iter().sum::<f64>() as f64 / req_sec.len() as f64;
        dbg!(requests_a_sec);

        info!("General benchmark results:");
        info!("     Total Succesful Requests Sent: {}", all_results.len());
        info!("     Average Requests/sec: {:.2}", requests_a_sec);
        info!("     Average Latency: {:?}", Duration::from_secs_f64(hist.mean() / (1000f64.powf(2.0))));
        info!("     Max Latency: {:?}", Duration::from_micros(hist.max()));
        info!("     Min Latency: {:?}", Duration::from_micros(hist.min()));
        info!("     Stdev Latency: {:?}", Duration::from_secs_f64(hist.stdev() / (1000f64.powf(2.0))));

        for (code, amount) in errors {
            warn!("     Got status {}: {}", code, amount);
        }

        let mut data: Vec<u32> = vec![0; all_sentence_length_latencies.keys().copied().max().unwrap_or(0)];
        for (length, durations) in all_sentence_length_latencies {
            if length == 0 {
                continue;
            }

            let avg: u64 = if durations.is_empty() {
                0u64
            } else {
                durations.iter()
                    .map(|v| v.as_millis() as u64)
                    .sum::<u64>() / durations.len() as u64
            };
            data[length-1] = avg as u32;
        }
        let max_latency = data.iter().copied().max().unwrap_or(0u32);
        let max_length = data.len() as u32;


        let root = BitMapBackend::new(&output, (1920, 1080)).into_drawing_area();

        root.fill(&WHITE)?;

        let mut chart = ChartBuilder::on(&root)
            .x_label_area_size(75)
            .y_label_area_size(75)
            .margin(5)
            .caption("Searching Latency Graph", ("sans-serif", 50.0))
            .build_cartesian_2d((1u32..max_length).into_segmented(), 0u32..max_latency)?;

        chart
            .configure_mesh()
            .disable_x_mesh()
            .bold_line_style(&WHITE.mix(0.5))
            .y_desc("Avg Latency (ms)")
            .x_desc("Sentence Length")
            .label_style(("sans-serif", 32))
            .axis_desc_style(("sans-serif", 48))
            .draw()?;

        chart.draw_series(
            Histogram::vertical(&chart)
                .style(RED.mix(0.5).filled())
                .data(data.iter().enumerate().map(|(y, x)| ((y+1) as u32, *x))),
        )?;

        // To avoid the IO failure being ignored silently, we manually call the present function
        let _ = root.present();
        info!("Result has been saved to {}", output);

        Ok(())
    }
}
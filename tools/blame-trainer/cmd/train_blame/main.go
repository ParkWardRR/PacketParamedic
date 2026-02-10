package main

import (
	"encoding/json"
	"flag"
	"fmt" // Re-added fmt
	"log"
	"math"
	"math/rand"
	"os"
	"time"
)

// Feature vector struct (Must align with Rust struct later)
type Feature struct {
	GwRttP50Ms    float64 `json:"gw_rtt_p50_ms"`
	GwRttP95Ms    float64 `json:"gw_rtt_p95_ms"`
	GwLossPct     float64 `json:"gw_loss_pct"`
	WanRttP50Ms   float64 `json:"wan_rtt_p50_ms"`
	WanRttP95Ms   float64 `json:"wan_rtt_p95_ms"`
	WanLossPct    float64 `json:"wan_loss_pct"`
	DeltaRttP50Ms float64 `json:"delta_rtt_p50_ms"` // Wan - Gw
	DnsMsP50      float64 `json:"dns_ms_p50"`
	DnsFailRate   float64 `json:"dns_fail_rate"`
	HttpFailRate  float64 `json:"http_fail_rate"`
	TcpFailRate   float64 `json:"tcp_fail_rate"`
	WanDownMbps   float64 `json:"wan_down_mbps"`
	WanUpMbps     float64 `json:"wan_up_mbps"`
}

// Convert Feature struct to Dense Vector for training
func (f Feature) ToVector() []float64 {
	return []float64{
		f.GwRttP50Ms, f.GwRttP95Ms, f.GwLossPct,
		f.WanRttP50Ms, f.WanRttP95Ms, f.WanLossPct,
		f.DeltaRttP50Ms,
		f.DnsMsP50, f.DnsFailRate,
		f.HttpFailRate, f.TcpFailRate,
		f.WanDownMbps, f.WanUpMbps,
	}
}

// Class Labels
const (
	LabelWifi   = 0
	LabelRouter = 1
	LabelIsp    = 2
)

var ClassNames = []string{"wifi", "router", "isp"}
var FeatureNames = []string{
	"gw_rtt_p50_ms", "gw_rtt_p95_ms", "gw_loss_pct",
	"wan_rtt_p50_ms", "wan_rtt_p95_ms", "wan_loss_pct",
	"delta_rtt_p50_ms",
	"dns_ms_p50", "dns_fail_rate",
	"http_fail_rate", "tcp_fail_rate",
	"wan_down_mbps", "wan_up_mbps",
}

// Model Artifact Structure (Export JSON)
type LogisticModel struct {
	FeatureNames []string    `json:"feature_names"`
	ClassNames   []string    `json:"class_names"`
	Weights      [][]float64 `json:"weights"` // [n_classes][n_features]
	Bias         []float64   `json:"bias"`    // [n_classes]
	Means        []float64   `json:"means"`   // For standardization
	Stds         []float64   `json:"stds"`    // For standardization
}

// Synthetic Data Generation
func generateSample(class int) Feature {
	// Helper for gaussian noise
	rng := func(mean, std float64) float64 {
		val := rand.NormFloat64()*std + mean
		if val < 0 {
			return 0
		}
		return val
	}

	var f Feature

	// Base "good" values
	wanBaseRtt := 15.0

	switch class {
	case LabelWifi:
		// Wi-Fi: Gateway metrics degrade significantly.
		gwRtt := rng(60.0, 10.0) // HIGH GW RTT
		f.GwRttP50Ms = gwRtt
		f.GwRttP95Ms = gwRtt * rng(1.5, 0.2)
		f.GwLossPct = rng(5.0, 2.0)

		// WAN is GW + Path, so WAN RTT is high
		f.WanRttP50Ms = gwRtt + rng(wanBaseRtt, 2.0)
		f.WanRttP95Ms = f.WanRttP50Ms * rng(1.2, 0.1)
		f.WanLossPct = f.GwLossPct + rng(1.0, 0.5)

		f.DeltaRttP50Ms = f.WanRttP50Ms - f.GwRttP50Ms // Should be ~15ms (normal path)

		f.DnsMsP50 = rng(20.0, 5.0)
		f.DnsFailRate = rng(0.01, 0.01)
		f.HttpFailRate = rng(0.01, 0.01)

	case LabelRouter:
		// Router: GW bad but less than Wifi, LOSS is king here.
		gwRtt := rng(25.0, 5.0)
		f.GwRttP50Ms = gwRtt
		f.GwRttP95Ms = gwRtt * rng(2.0, 0.5)
		f.GwLossPct = rng(15.0, 5.0) // HIGH LOSS

		f.WanRttP50Ms = gwRtt + rng(wanBaseRtt, 5.0)
		f.WanLossPct = f.GwLossPct + rng(2.0, 1.0)
		f.DeltaRttP50Ms = f.WanRttP50Ms - f.GwRttP50Ms

		f.DnsMsP50 = rng(100.0, 50.0)
		f.DnsFailRate = rng(0.25, 0.1) // High failure
		f.HttpFailRate = rng(0.20, 0.1)
		f.TcpFailRate = rng(0.15, 0.1)

	case LabelIsp:
		// ISP: Gateway is pristine. WAN metrics degrade. Delta RTT is huge.
		f.GwRttP50Ms = rng(2.0, 0.5) // PRISTINE GW
		f.GwRttP95Ms = rng(3.0, 1.0)
		f.GwLossPct = rng(0.0, 0.1)

		f.WanRttP50Ms = rng(120.0, 30.0) // HIGH LATENCY UPSTREAM
		f.WanRttP95Ms = f.WanRttP50Ms * rng(1.5, 0.2)
		f.WanLossPct = rng(5.0, 2.0)

		f.DeltaRttP50Ms = f.WanRttP50Ms - f.GwRttP50Ms // HUGE DELTA

		f.DnsMsP50 = rng(50.0, 10.0)
		f.DnsFailRate = rng(0.05, 0.02)
		f.HttpFailRate = rng(0.05, 0.02)
	}

	return f
}

// simple Softmax Regression training (SGD)
func trainSoftmax(features [][]float64, labels []int, epochs int, lr float64) (*LogisticModel, float64) {
	nSamples := len(features)
	nFeats := len(features[0])
	nClasses := 3

	// 1. Compute standardization parameters
	means := make([]float64, nFeats)
	stds := make([]float64, nFeats)

	for _, samp := range features {
		for j, val := range samp {
			means[j] += val
		}
	}
	for j := 0; j < nFeats; j++ {
		means[j] /= float64(nSamples)
	}

	for _, samp := range features {
		for j, val := range samp {
			stds[j] += math.Pow(val-means[j], 2)
		}
	}
	for j := 0; j < nFeats; j++ {
		stds[j] = math.Sqrt(stds[j] / float64(nSamples))
		if stds[j] < 1e-6 {
			stds[j] = 1.0
		} // Prevent div/0
	}

	// 2. Standardize features
	normFeatures := make([][]float64, nSamples)
	for i, samp := range features {
		norm := make([]float64, nFeats)
		for j, val := range samp {
			norm[j] = (val - means[j]) / stds[j]
		}
		normFeatures[i] = norm
	}

	// 3. Initialize weights (Xavier/GLOROT)
	weights := make([][]float64, nClasses)
	bias := make([]float64, nClasses)
	limit := math.Sqrt(6.0 / float64(nFeats+nClasses))

	for k := 0; k < nClasses; k++ {
		weights[k] = make([]float64, nFeats)
		for j := 0; j < nFeats; j++ {
			weights[k][j] = rand.Float64()*2*limit - limit
		}
	}

	// 4. Training Loop (SGD)
	for epoch := 0; epoch < epochs; epoch++ {
		// Shuffle (omitted for brevity, assume random enough order)

		for i := 0; i < nSamples; i++ {
			x := normFeatures[i]
			y := labels[i]

			// Forward pass: Scores z_k = w_k * x + b_k
			scores := make([]float64, nClasses)
			maxScore := -1e9
			for k := 0; k < nClasses; k++ {
				dot := 0.0
				for j := 0; j < nFeats; j++ {
					dot += weights[k][j] * x[j]
				}
				scores[k] = dot + bias[k]
				if scores[k] > maxScore {
					maxScore = scores[k]
				}
			}

			// Softmax
			sumExp := 0.0
			probs := make([]float64, nClasses)
			for k := 0; k < nClasses; k++ {
				probs[k] = math.Exp(scores[k] - maxScore) // Stable softmax
				sumExp += probs[k]
			}
			for k := 0; k < nClasses; k++ {
				probs[k] /= sumExp
			}

			// Backward pass
			// grad_z_k = p_k - (1 if k==y else 0)
			for k := 0; k < nClasses; k++ {
				grad := probs[k]
				if k == y {
					grad -= 1.0
				}

				// Update bias
				bias[k] -= lr * grad

				// Update weights
				for j := 0; j < nFeats; j++ {
					weights[k][j] -= lr * grad * x[j]
				}
			}
		}
	}

	// Calculate final accuracy
	correct := 0
	for i := 0; i < nSamples; i++ {
		x := normFeatures[i]
		y := labels[i]

		scores := make([]float64, nClasses)
		for k := 0; k < nClasses; k++ {
			dot := 0.0
			for j := 0; j < nFeats; j++ {
				dot += weights[k][j] * x[j]
			}
			scores[k] = dot + bias[k]
		}

		bestK := -1
		bestScore := -1e9
		for k, s := range scores {
			if s > bestScore {
				bestScore = s
				bestK = k
			}
		}
		if bestK == y {
			correct++
		}
	}

	accuracy := float64(correct) / float64(nSamples)

	return &LogisticModel{
		FeatureNames: FeatureNames,
		ClassNames:   ClassNames,
		Weights:      weights,
		Bias:         bias,
		Means:        means,
		Stds:         stds,
	}, accuracy
}

func main() {
	nSamples := flag.Int("n", 50000, "Number of synthetic samples")
	outPath := flag.String("out", "./blame_lr.json", "Output JSON path")
	epochs := flag.Int("epochs", 100, "Training epochs")
	lr := flag.Float64("lr", 0.01, "Learning rate")
	flag.Parse()

	rand.Seed(time.Now().UnixNano())

	log.Printf("Generating %d synthetic samples...", *nSamples)

	var features [][]float64
	var labels []int

	// Balanced dataset
	for i := 0; i < *nSamples; i++ {
		label := rand.Intn(3)
		sample := generateSample(label)
		features = append(features, sample.ToVector())
		labels = append(labels, label)
	}

	log.Println("Training Softmax Regression...")
	model, acc := trainSoftmax(features, labels, *epochs, *lr)

	log.Printf("Training complete. Accuracy: %.2f%%", acc*100)

	bytes, err := json.MarshalIndent(model, "", "  ")
	if err != nil {
		log.Fatal(err)
	}

	if err := os.WriteFile(*outPath, bytes, 0644); err != nil {
		log.Fatal(err)
	}

	log.Printf("Model saved to %s", *outPath)

	// Debug: Print weights
	fmt.Println("Bias:", model.Bias)
	fmt.Println("Weights (Wifi):", model.Weights[0])
	fmt.Println("Weights (ISP):", model.Weights[2])
	fmt.Println("Means:", model.Means)
}

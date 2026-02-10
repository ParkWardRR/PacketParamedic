package main

import (
	"encoding/json"
	"flag"
	"fmt"
	"log"
	"math"
	"os"
)

type LogisticModel struct {
	FeatureNames []string    `json:"feature_names"`
	ClassNames   []string    `json:"class_names"`
	Weights      [][]float64 `json:"weights"` // [n_classes][n_features]
	Bias         []float64   `json:"bias"`    // [n_classes]
	Means        []float64   `json:"means"`   // For standardization
	Stds         []float64   `json:"stds"`    // For standardization
}

func main() {
	modelPath := flag.String("model", "blame_lr.json", "Path to model JSON")
	flag.Parse()

	bytes, err := os.ReadFile(*modelPath)
	if err != nil {
		log.Fatalf("Failed to read model: %v", err)
	}

	var model LogisticModel
	if err := json.Unmarshal(bytes, &model); err != nil {
		log.Fatalf("Failed to parse model: %v", err)
	}

	// Hardcoded example (ISP Failure Pattern)
	// GW ok (2ms), WAN bad (100ms), Delta large (98ms), minimal loss, some DNS slow
	feats := []float64{
		2.0, 3.0, 0.0, // GW: nice
		100.0, 150.0, 5.0, // WAN: slow
		98.0,       // Delta: huge
		50.0, 0.05, // DNS: slightly slow/fail
		0.05, 0.0, // HTTP/TCP
		100.0, 10.0, // Throughput
	}

	// 1. Standardize
	normFeats := make([]float64, len(feats))
	for i, v := range feats {
		normFeats[i] = (v - model.Means[i]) / model.Stds[i]
	}

	// 2. Predict Probabilities (Softmax)
	var scores []float64
	maxScore := -1e9

	for k := 0; k < len(model.ClassNames); k++ {
		score := 0.0
		for j, w := range model.Weights[k] {
			score += w * normFeats[j]
		}
		score += model.Bias[k]
		scores = append(scores, score)
		if score > maxScore {
			maxScore = score
		}
	}

	var probs []float64
	sumExp := 0.0
	for _, s := range scores {
		p := math.Exp(s - maxScore)
		probs = append(probs, p)
		sumExp += p
	}

	fmt.Println("Prediction for synthetic ISP-failure sample:")
	for k, name := range model.ClassNames {
		prob := probs[k] / sumExp
		fmt.Printf("  %s: %.4f\n", name, prob)
	}
}

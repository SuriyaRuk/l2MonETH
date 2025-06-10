package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"math/rand"
	"net/http"
	"os"
	"time"
)

type BlockNumberResponse struct {
	JSONRPC string `json:"jsonrpc"`
	ID      int    `json:"id"`
	Result  string `json:"result"` // Block number as a hex string (e.g., "0x5B9AC0")
}

func getBlockNumber(rpcURL string) (int64, error) {
	// JSON-RPC payload
	currentTime := time.Now()
	id := rand.Intn(100)
	payload := map[string]interface{}{
		"jsonrpc": "2.0",
		"method":  "eth_blockNumber",
		"params":  []interface{}{},
		"id":      id,
	}

	// Convert payload to JSON
	jsonData, err := json.Marshal(payload)
	if err != nil {
		fmt.Println("Error marshalling JSON:", err)
		return 0, err
	}

	rpc := rpcURL
	if rpc == "" {
		rpc = "http://127.0.0.1:8545"
	}

	// Send the request
	resp, err := http.Post(rpc, "application/json", bytes.NewBuffer(jsonData))
	if err != nil {
		fmt.Println("Error sending request:", err)
		return 0, err
	}
	defer resp.Body.Close()

	// Read the response
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		fmt.Println("Error reading response:", err)
		return 0, err
	}

	// Unmarshal the response into BlockNumberResponse struct
	var blockNumberResp BlockNumberResponse
	err = json.Unmarshal(body, &blockNumberResp)
	if err != nil {
		fmt.Println("Error unmarshalling response:", err)
		return 0, err
	}

	// Print the block number in hex format
	fmt.Println("currentTime:", currentTime, "ID ", id, "Block Number (Hex):", blockNumberResp.Result)

	// Optionally, convert the hex block number to a decimal
	var blockNumber int64
	_, err = fmt.Sscanf(blockNumberResp.Result, "0x%x", &blockNumber)
	if err != nil {
		fmt.Println("Error converting hex to decimal:", err)
		return 0, err
	}

	// Print the block number in decimal format
	fmt.Println("currentTime:", currentTime, "ID ", id, "Block Number (Decimal):", blockNumber)
	return blockNumber, nil

}

type BlockResponse struct {
	BlockNumberHex     string `json:"block_number_hex"`
	BlockNumberDecimal int64  `json:"block_number_decimal"`
	Status             string `json:"status"`
}

func checkSync(w http.ResponseWriter, r *http.Request) {
	rpcURL := r.URL.Query().Get("rpc")
	
	blockNumberFirst, err := getBlockNumber(rpcURL)
	if err != nil {
		w.WriteHeader(500)
		return
	}

	time.Sleep(30 * time.Second)

	blockNumberSecond, err := getBlockNumber(rpcURL)
	if err != nil {
		w.WriteHeader(500)
		return
	}

	blockNumber := blockNumberSecond - blockNumberFirst
	
	// Return both hex and decimal formats
	response := BlockResponse{
		BlockNumberHex:     fmt.Sprintf("0x%x", blockNumberSecond),
		BlockNumberDecimal: blockNumberSecond,
	}
	
	if blockNumber != 0 {
		response.Status = "synced"
		w.WriteHeader(200)
	} else {
		response.Status = "not_synced"
		w.WriteHeader(500)
	}
	
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(response)
}

func main() {

	http.HandleFunc("/", checkSync)
	port := os.Getenv("PORT")
	if port == "" {
		port = "9999"
	}

	fmt.Printf("Starting server on port %s\n", port)

	err := http.ListenAndServe(":"+port, nil)
	if err != nil {
		panic(err)
	}
}

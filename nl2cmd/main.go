package main

import (
	"context"
	"errors"
	"log"
	"net/http"
	"os"

	"github.com/firebase/genkit/go/ai"
	"github.com/firebase/genkit/go/genkit"
	"github.com/firebase/genkit/go/plugins/compat_oai/anthropic"
	"github.com/firebase/genkit/go/plugins/server"
	"github.com/openai/openai-go/option"
)

type NL2Cmd struct {
	NL2Cmd string `json:"nl2cmd" jsonschema:"description=Describes Terminal command in natrual language"`
}

type CmdResp struct {
	Cmd string `json:"cmd"`
	Runnable bool `json:"runnable"`
}

func main() {
	ctx := context.Background()
	// Define claude plugin.
	claude := &anthropic.Anthropic{
		Opts: []option.RequestOption{
			option.WithAPIKey(os.Getenv("CLAUDE_API_KEY")),
		},
	}
	// Initialize genkit with claude plugin and promts dir.
	g := genkit.Init(ctx,
		genkit.WithPlugins(claude),
		genkit.WithPromptDir("./prompts"),
	)
	// Define flow
	genkit.DefineFlow(g, "nl2CmdFlow", func(ctx context.Context, nl2cmd *NL2Cmd) (*CmdResp, error) {
		log.Printf("Received input: %+v", nl2cmd)
		
		nl2cmdPromt := genkit.LookupPrompt(g, "nl2cmd")
		if nl2cmdPromt == nil {
			return nil, errors.New("Prompt not found: nl2cmd")
		}

		log.Printf("Executing prompt...")
		res, err := nl2cmdPromt.Execute(ctx, ai.WithInput(nl2cmd))
		if err != nil {
			log.Printf("Execute error: %v", err)
			return nil, err
		}

		var cmd CmdResp
		if err := res.Output(&cmd); err != nil {
			log.Printf("Error parsing response: %v", err)
			log.Printf("Raw text that failed to parse: %s", res.Text())
			return nil, err
		}

		log.Printf("Successfully parsed response: %+v", cmd)
		return &cmd, nil
	})
	
	// Server the flow on a http
	mux := http.NewServeMux()
	for _, a := range genkit.ListFlows(g) {
		mux.HandleFunc("POST /"+a.Name(), genkit.Handler(a))
	}
	log.Fatal(server.Start(ctx, "127.0.0.1:3400", mux))
}

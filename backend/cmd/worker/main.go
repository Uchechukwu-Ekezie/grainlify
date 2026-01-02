package main

import (
	"context"
	"log/slog"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/jagadeesh/grainlify/backend/internal/bus/natsbus"
	"github.com/jagadeesh/grainlify/backend/internal/config"
	"github.com/jagadeesh/grainlify/backend/internal/db"
	"github.com/jagadeesh/grainlify/backend/internal/ingest"
	"github.com/jagadeesh/grainlify/backend/internal/syncjobs"
	"github.com/jagadeesh/grainlify/backend/internal/worker"
)

func main() {
	config.LoadDotenv()
	cfg := config.Load()

	logger := slog.New(slog.NewTextHandler(os.Stdout, &slog.HandlerOptions{
		Level: cfg.LogLevel(),
	}))
	slog.SetDefault(logger)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	if cfg.DBURL == "" {
		slog.Error("DB_URL is required")
		os.Exit(1)
	}
	d, err := db.Connect(ctx, cfg.DBURL)
	if err != nil {
		slog.Error("db connect failed", "error", err)
		os.Exit(1)
	}
	defer d.Close()

	if cfg.NATSURL == "" {
		slog.Error("NATS_URL is required to run workers")
		os.Exit(1)
	}

	b, err := natsbus.Connect(cfg.NATSURL)
	if err != nil {
		slog.Error("nats connect failed", "error", err)
		os.Exit(1)
	}
	defer b.Close()

	ingestor := &ingest.GitHubWebhookIngestor{Pool: d.Pool}
	consumer := &worker.GitHubWebhookConsumer{Ingest: ingestor}
	if err := consumer.Subscribe(ctx, b.Conn(), "patchwork-workers"); err != nil {
		slog.Error("subscribe failed", "error", err)
		os.Exit(1)
	}

	// Also run the DB-backed sync job worker loop.
	syncWorker := syncjobs.New(cfg, d.Pool)
	go func() { _ = syncWorker.Run(ctx) }()

	slog.Info("worker started")

	sigCh := make(chan os.Signal, 2)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)
	<-sigCh
	slog.Info("worker shutting down")
	cancel()
	time.Sleep(300 * time.Millisecond)
}








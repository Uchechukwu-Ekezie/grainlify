package migrate

import (
	"context"
	"fmt"
	"log/slog"

	"github.com/golang-migrate/migrate/v4"
	"github.com/golang-migrate/migrate/v4/database/postgres"
	"github.com/golang-migrate/migrate/v4/source/iofs"
	"github.com/jackc/pgx/v5/pgxpool"
	"github.com/jackc/pgx/v5/stdlib"

	"github.com/jagadeesh/grainlify/backend/migrations"
)

func Up(ctx context.Context, pool *pgxpool.Pool) error {
	if pool == nil {
		return fmt.Errorf("db pool is nil")
	}

	slog.Info("loading embedded migration files")
	src, err := iofs.New(migrations.FS, ".")
	if err != nil {
		slog.Error("failed to load embedded migrations",
			"error", err,
			"error_type", fmt.Sprintf("%T", err),
		)
		return fmt.Errorf("open embedded migrations: %w", err)
	}
	slog.Info("embedded migrations loaded")

	slog.Info("opening database connection for migrations")
	sqlDB := stdlib.OpenDB(*pool.Config().ConnConfig)
	defer sqlDB.Close()

	slog.Info("creating postgres migration driver")
	db, err := postgres.WithInstance(sqlDB, &postgres.Config{})
	if err != nil {
		slog.Error("failed to create postgres migration driver",
			"error", err,
			"error_type", fmt.Sprintf("%T", err),
		)
		return fmt.Errorf("create postgres migration driver: %w", err)
	}

	slog.Info("creating migrator instance")
	m, err := migrate.NewWithInstance("iofs", src, "postgres", db)
	if err != nil {
		slog.Error("failed to create migrator",
			"error", err,
			"error_type", fmt.Sprintf("%T", err),
		)
		return fmt.Errorf("create migrator: %w", err)
	}
	defer func() {
		slog.Info("closing migrator")
		_, _ = m.Close()
	}()

	// Check current version before migrating
	version, dirty, err := m.Version()
	if err != nil && err != migrate.ErrNilVersion {
		slog.Warn("could not get current migration version",
			"error", err,
		)
	} else {
		slog.Info("current migration version",
			"version", version,
			"dirty", dirty,
		)
	}

	// migrate.Up() is not context-aware; we still accept ctx for future evolutions.
	_ = ctx

	slog.Info("running database migrations")
	if err := m.Up(); err != nil && err != migrate.ErrNoChange {
		slog.Error("migration failed",
			"error", err,
			"error_type", fmt.Sprintf("%T", err),
		)
		return err
	}

	if err == migrate.ErrNoChange {
		slog.Info("migrations up to date, no changes needed")
	} else {
		// Get version after migration
		newVersion, _, verErr := m.Version()
		if verErr == nil {
			slog.Info("migrations completed successfully",
				"new_version", newVersion,
			)
		} else {
			slog.Info("migrations completed successfully")
		}
	}

	return nil
}



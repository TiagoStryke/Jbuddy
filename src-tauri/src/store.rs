//! Persistência local em SQLite (`~/Library/Application Support/Jbuddy/jbuddy.db`).
//!
//! Guarda tempo por dia e por hora-do-dia. Os buckets por hora já deixam o
//! heatmap de produtividade (Fase 3) barato de montar depois.

use std::path::PathBuf;

use rusqlite::{params, Connection};

use crate::tracker::ActivityState;

pub struct Store {
    conn: Connection,
}

/// Totais (trabalhando, ocioso, ausente) em segundos.
pub type Totals = (i64, i64, i64);

impl Store {
    /// Abre (criando o diretório e as tabelas se preciso).
    pub fn open() -> rusqlite::Result<Self> {
        let dir = data_dir();
        std::fs::create_dir_all(&dir).ok();
        let conn = Connection::open(dir.join("jbuddy.db"))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS daily (
                date         TEXT PRIMARY KEY,
                working_secs INTEGER NOT NULL DEFAULT 0,
                idle_secs    INTEGER NOT NULL DEFAULT 0,
                away_secs    INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS hourly (
                date         TEXT NOT NULL,
                hour         INTEGER NOT NULL,
                working_secs INTEGER NOT NULL DEFAULT 0,
                idle_secs    INTEGER NOT NULL DEFAULT 0,
                away_secs    INTEGER NOT NULL DEFAULT 0,
                PRIMARY KEY (date, hour)
            );",
        )?;
        Ok(Self { conn })
    }

    /// Credita `secs` no bucket do dia e da hora, na coluna do estado atual.
    pub fn credit(
        &self,
        date: &str,
        hour: i64,
        state: ActivityState,
        secs: i64,
    ) -> rusqlite::Result<()> {
        let (w, i, a) = match state {
            ActivityState::Working => (secs, 0, 0),
            ActivityState::Idle => (0, secs, 0),
            ActivityState::Away => (0, 0, secs),
        };
        // Os dois UPSERTs numa transação: ou ambos aplicam, ou nenhum —
        // mantém `daily` e `hourly` sempre consistentes.
        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO daily (date, working_secs, idle_secs, away_secs)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(date) DO UPDATE SET
                working_secs = working_secs + ?2,
                idle_secs    = idle_secs + ?3,
                away_secs    = away_secs + ?4",
            params![date, w, i, a],
        )?;
        tx.execute(
            "INSERT INTO hourly (date, hour, working_secs, idle_secs, away_secs)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(date, hour) DO UPDATE SET
                working_secs = working_secs + ?3,
                idle_secs    = idle_secs + ?4,
                away_secs    = away_secs + ?5",
            params![date, hour, w, i, a],
        )?;
        tx.commit()
    }

    /// Totais do dia. Retorna `(0, 0, 0)` se ainda não há linha.
    pub fn today_totals(&self, date: &str) -> rusqlite::Result<Totals> {
        self.conn
            .query_row(
                "SELECT working_secs, idle_secs, away_secs FROM daily WHERE date = ?1",
                params![date],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .or_else(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => Ok((0, 0, 0)),
                other => Err(other),
            })
    }
}

fn data_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join("Library/Application Support/Jbuddy")
}

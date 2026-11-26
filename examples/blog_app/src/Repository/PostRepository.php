<?php

declare(strict_types=1);
// Target: PHP 8.4

namespace TypePhp\BlogApp\Repository;

use PDO;
use PDOStatement;

class PostRepository
{
    private PDO $pdo;

    public function __construct(string $dbPath)
    {
        $this->pdo = new PDO('sqlite:' . $dbPath);
        $this->pdo->setAttribute(PDO::ATTR_ERRMODE, PDO::ERRMODE_EXCEPTION);
        $this->pdo->exec('CREATE TABLE IF NOT EXISTS posts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                slug TEXT NOT NULL,
                content TEXT NOT NULL,
                published INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )');
    }

    /**
     * @return array<int, mixed>
     */
    public function findPublished(): array
    {
        $stmt = $this->pdo->prepare('SELECT id, title, slug, content, published, created_at FROM posts WHERE published = 1 ORDER BY id DESC');
        $stmt->execute([]);
        return $stmt->fetchAll(PDO::FETCH_ASSOC);
    }

    /**
     * @return array<int, mixed>
     */
    public function findAll(): array
    {
        $stmt = $this->pdo->prepare('SELECT id, title, slug, content, published, created_at FROM posts ORDER BY id DESC');
        $stmt->execute([]);
        return $stmt->fetchAll(PDO::FETCH_ASSOC);
    }

    public function findById(int $id): mixed
    {
        $stmt = $this->pdo->prepare('SELECT id, title, slug, content, published, created_at FROM posts WHERE id = ?');
        $stmt->execute([$id]);
        return $stmt->fetch(PDO::FETCH_ASSOC);
    }

    public function create(string $title, string $slug, string $content, int $published): mixed
    {
        $stmt = $this->pdo->prepare('INSERT INTO posts (title, slug, content, published) VALUES (?, ?, ?, ?)');
        $stmt->execute([$title, $slug, $content, $published]);
        $newId = intval($this->pdo->lastInsertId());
        return $this->findById($newId);
    }

    public function update(int $id, string $title, string $slug, string $content, int $published): bool
    {
        $stmt = $this->pdo->prepare('UPDATE posts SET title = ?, slug = ?, content = ?, published = ? WHERE id = ?');
        $stmt->execute([$title, $slug, $content, $published, $id]);
        return $stmt->rowCount() > 0;
    }

    public function delete(int $id): bool
    {
        $stmt = $this->pdo->prepare('DELETE FROM posts WHERE id = ?');
        $stmt->execute([$id]);
        return $stmt->rowCount() > 0;
    }
}

<?php

declare(strict_types=1);

/**
 * @template T
 * @param T $payload
 */
function sendJson(int $status, mixed $payload): void
{
    http_response_code($status);
    header('Content-Type: application/json; charset=utf-8');
    echo json_encode($payload);
}

function sendError(int $status, string $message): void
{
    /** @var array{ok: bool, error: string} $err */
    $err = ['ok' => false, 'error' => $message];
    sendJson($status, $err);
}

class TaskRepository
{
    private PDO $pdo;

    public function __construct(string $dbPath)
    {
        $this->pdo = new PDO('sqlite:' . $dbPath);
        $this->pdo->setAttribute(PDO::ATTR_ERRMODE, PDO::ERRMODE_EXCEPTION);
        $this->pdo->exec('CREATE TABLE IF NOT EXISTS tasks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                completed INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            )');
    }

    /**
     * @return list<mixed>
     */
    public function findAll(): array
    {
        $stmt = $this->pdo->query('SELECT id, title, completed, created_at FROM tasks ORDER BY id DESC');
        return $stmt->fetchAll(PDO::FETCH_ASSOC);
    }

    public function findById(int $id): mixed
    {
        $stmt = $this->pdo->prepare('SELECT id, title, completed, created_at FROM tasks WHERE id = ?');
        $stmt->execute([$id]);
        return $stmt->fetch(PDO::FETCH_ASSOC);
    }

    public function create(string $title): mixed
    {
        $stmt = $this->pdo->prepare('INSERT INTO tasks (title, completed) VALUES (?, 0)');
        $stmt->execute([$title]);
        $newId = intval($this->pdo->lastInsertId());
        return $this->findById($newId);
    }

    public function update(int $id, string $title, int $completed): bool
    {
        $stmt = $this->pdo->prepare('UPDATE tasks SET title = ?, completed = ? WHERE id = ?');
        $stmt->execute([$title, $completed, $id]);
        return $stmt->rowCount() > 0;
    }

    public function delete(int $id): bool
    {
        $stmt = $this->pdo->prepare('DELETE FROM tasks WHERE id = ?');
        $stmt->execute([$id]);
        return $stmt->rowCount() > 0;
    }
}

function readJsonBody(): mixed
{
    $raw = file_get_contents('php://input');
    if (trim($raw) === '') {
        return [];
    }
    return json_decode($raw, true);
}

$repo = new TaskRepository(__DIR__ . '/tasks.db');
$method = strtoupper(strval($_SERVER['REQUEST_METHOD'] ?? 'GET'));
$uri = strval($_SERVER['REQUEST_URI'] ?? '/tasks');
$rawPath = strval(parse_url($uri, PHP_URL_PATH));
$trimmed = trim($rawPath, '/');
/** @var list<string> $segments */
$segments = explode('/', $trimmed);
if ($trimmed === '' || $segments[0] !== 'tasks') {
    sendError(404, 'Endpoint not found. Use /tasks or /tasks/{id}');
    return;
}
$segCount = count($segments);
if ($segCount === 1) {
    if ($method === 'GET') {
        /** @var list<mixed> $tasks */
        $tasks = $repo->findAll();
        /** @var array{ok: bool, data: T} $res */
        $res = ['ok' => true, 'data' => $tasks];
        sendJson(200, $res);
    } elseif ($method === 'POST') {
        $body = readJsonBody();
        if (!is_array($body) || !array_key_exists('title', $body)) {
            sendError(400, 'Field "title" is required');
            return;
        }
        $title = trim(strval($body['title']));
        if ($title === '') {
            sendError(400, 'Field "title" cannot be empty');
            return;
        }
        $created = $repo->create($title);
        /** @var array{ok: bool, data: T} $res */
        $res = ['ok' => true, 'data' => $created];
        sendJson(201, $res);
    } else {
        sendError(405, 'Method not allowed');
    }
} elseif ($segCount === 2) {
    $id = intval($segments[1]);
    if ($id <= 0) {
        sendError(400, 'Invalid task ID');
        return;
    }
    if ($method === 'GET') {
        $task = $repo->findById($id);
        if ($task === false) {
            sendError(404, 'Task not found');
            return;
        }
        /** @var array{ok: bool, data: T} $res */
        $res = ['ok' => true, 'data' => $task];
        sendJson(200, $res);
    } elseif ($method === 'PUT') {
        $existing = $repo->findById($id);
        if ($existing === false) {
            sendError(404, 'Task not found');
            return;
        }
        $body = readJsonBody();
        if (!is_array($body)) {
            sendError(400, 'Invalid JSON body');
            return;
        }
        $title = strval($existing['title']);
        if (array_key_exists('title', $body)) {
            $title = trim(strval($body['title']));
        }
        $completed = intval($existing['completed']);
        if (array_key_exists('completed', $body)) {
            $completed = intval($body['completed']);
        }
        if ($title === '') {
            sendError(400, 'Field "title" cannot be empty');
            return;
        }
        $repo->update($id, $title, $completed);
        $updated = $repo->findById($id);
        /** @var array{ok: bool, data: T} $res */
        $res = ['ok' => true, 'data' => $updated];
        sendJson(200, $res);
    } elseif ($method === 'DELETE') {
        $deleted = $repo->delete($id);
        if (!$deleted) {
            sendError(404, 'Task not found');
            return;
        }
        /** @var array{ok: bool, data: T} $res */
        $res = ['ok' => true, 'data' => 'Deleted'];
        sendJson(200, $res);
    } else {
        sendError(405, 'Method not allowed');
    }
} else {
    sendError(404, 'Endpoint not found');
}

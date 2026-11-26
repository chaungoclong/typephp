<?php
declare(strict_types=1);

interface Printable {
    public function summary(): string;
}

enum Role: string {
    case Admin = 'admin';
    case Member = 'member';
}

/**
 * @template T
 */
class Box {
    private mixed $value;
    public function __construct(mixed $value) {
        $this->value = $value;
    }
    public function get(): mixed {
        return $this->value;
    }
}

class Account implements Printable {
    public string $username;
    public Role $role;
    public function __construct(string $username, Role $role) {
        $this->username = $username;
        $this->role = $role;
    }
    public function summary(): string {
        return $this->username;
    }
}

/**
 * @template T
 */
function identity(mixed $x): mixed {
    return $x;
}

function formatId(int|string|null $id): string {
    if ($id === null) {
        return 'none';
    }
    if (is_int($id)) {
        $num = $id;
        return 'ID#' . (string)$num;
    }
    return $id;
}

$count = identity(42);
$box = new Box('TypePHP');
$title = $box->get();
$user = ['id' => 1, 'name' => 'Long'];
$acc = new Account($user['name'], Role::Admin);
$double = fn(int $n): int => $n * 2;
$res = $double($count);

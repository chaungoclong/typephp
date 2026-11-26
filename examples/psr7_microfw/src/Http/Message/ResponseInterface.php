<?php

declare(strict_types=1);
// Target: PHP 8.4

namespace TypePhp\MicroFw\Http\Message;

require_once __DIR__ . '/MessageInterface.php';

interface ResponseInterface extends MessageInterface
{
    public function getStatusCode(): int;
    public function getReasonPhrase(): string;
    public function withStatus(int $code, string $reasonPhrase = ''): ResponseInterface;
}

<?php

declare(strict_types=1);
// Target: PHP 8.4

namespace TypePhp\MicroFw\Http\Message;

require_once __DIR__ . '/UriInterface.php';

class Uri implements UriInterface
{
    private string $scheme;
    private string $host;
    private string $path;
    private string $query;

    public function __construct(string $scheme, string $host, string $path, string $query = '')
    {
        $this->scheme = $scheme;
        $this->host = $host;
        $this->path = $path;
        $this->query = $query;
    }

    public function getScheme(): string
    {
        return $this->scheme;
    }

    public function getHost(): string
    {
        return $this->host;
    }

    public function getPath(): string
    {
        return $this->path;
    }

    public function getQuery(): string
    {
        return $this->query;
    }
}

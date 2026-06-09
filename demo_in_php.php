route = [S]
current = S
remaining = W
legs = []

while remaining is not empty:

    best_next = null
    best_distance = ∞

    for each point p in remaining:
        if dist(current, p) < best_distance:
            best_distance = dist(current, p)
            best_next = p

    if best_next is null:
        break

    legs.append({
        from: current,
        to: best_next,
        distance: dist(current, best_next)
    })

    route.append(best_next)
    current = best_next
    remove best_next from remaining


# final leg to destination
legs.append({
    from: current,
    to: D,
    distance: dist(current, D)
})

route.append(D)

return route, legs

// example reponse:

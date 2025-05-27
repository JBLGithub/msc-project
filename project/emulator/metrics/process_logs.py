from matplotlib import pyplot as plt
from datetime import datetime
import numpy as np
import seaborn as sns
import pandas as pd
import json
from scipy.stats import linregress

today_date = datetime.today().strftime('%Y-%m-%d')

def path_discovery_convergence():
    with open("packet_path_discovery.txt", "r") as file:
        flip = False
        current = 0
        timings = {}

        for line in file:
            if "DISCOVERY_STARTED" in line or "DISCOVERY_COMPLETED" in line:
                metric = line.strip()
                timestamp = int(metric.split(";")[2])
                nb_routers = int(metric.split(";")[4])

                if "DISCOVERY_STARTED" in metric:
                    current = timestamp
                    flip = True
                if flip and "DISCOVERY_COMPLETED" in metric:
                    result = int((timestamp - current))
                    if nb_routers in timings:
                        timings[nb_routers].append(result)
                    else:
                        timings[nb_routers] = [result]
                    flip = False

        keys = list(timings.keys())
        data = [np.array(timings[key]) / 1000 for key in keys]

        flierprops = dict(marker='.', markerfacecolor='blue', markersize=2)
        medianprops = dict(linestyle='-', color='orange', linewidth=2)
        meanprops = dict(linestyle='--', linewidth=2, color='g')

        plt.figure(figsize=(12, 8))
        plt.boxplot(data, tick_labels=keys, flierprops=flierprops, medianprops=medianprops, meanprops=meanprops, showmeans=True, meanline=True)
        plt.title(f'path discovery convergence vs routers: {today_date}, 1-10 routers in series (50 measurement bins, 5th/95th whiskers), st-andrews cs network', fontsize=10)
        plt.xlabel('routers')
        plt.ylabel('time (ms)')
        max_value = max(max(d) for d in data)
        plt.yticks(range(0, int(max_value) + 1, 1))
        plt.grid(axis='y', linestyle='-', alpha=0.7)
        plt.subplots_adjust(left=0.06, right=0.97, top=0.94, bottom=0.09)
        plt.savefig("plot_path_discovery.png")
        plt.close()

        means = [np.mean(d) for d in data]
        x = np.arange(1, len(means) + 1)
        slope, intercept, _, _, _ = linregress(x, means)
        line_of_best_fit = slope * x + intercept

        plt.figure(figsize=(12, 4))
        plt.plot(x, means, 'o', label='mean path convergence')
        plt.plot(x, line_of_best_fit, 'orange', label=f'Y = {slope:.2f}X {intercept:.2f}')
        plt.title(f'mean path discovery convergence vs routers ({today_date}), 1-10 routers in series (50 measurement bins), st-andrews cs network', fontsize=10)
        plt.xlabel('routers')
        plt.ylabel('time (μs)')
        plt.legend()
        plt.grid(axis='y', linestyle='-', alpha=0.7)
        plt.xticks(np.arange(1, len(means) + 1, 1))
        max_y = int(max(means))
        plt.yticks(np.arange(0, max_y + 1, 1))
        plt.subplots_adjust(left=0.06, right=0.97, top=0.9, bottom=0.13)
        plt.savefig("plot_path_discovery_mean.png")
        plt.close()


def packet_overhead_single():
    with open("packet_overhead_single.txt", "r") as file:

        result = {}

        for line in file:
            metric = line.strip()

            if "PCB" in metric:

                metric_split = metric.split(";")
                key = int(metric_split[4])
                pcb_string = metric_split[5]
                pcb = json.loads(pcb_string)

                overhead_count = 0
                payload_count = 0
                overhead_count += int(pcb.get("nd_solicitation_jcmp_tx"))
                overhead_count += int(pcb.get("nd_advertisement_jcmp_tx"))
                overhead_count += int(pcb.get("dns_fqdn_query_jcmp_tx"))
                overhead_count += int(pcb.get("dns_fqdn_response_jcmp_tx"))
                overhead_count += int(pcb.get("dns_ilv_query_jcmp_tx"))
                overhead_count += int(pcb.get("dns_ilv_response_jcmp_tx"))
                overhead_count += int(pcb.get("router_request_jcmp_tx"))
                overhead_count += int(pcb.get("router_response_jcmp_tx"))
                
                payload_count += int(pcb.get("data_request_tx"))

                if key in result:
                    result[key]["overhead"] += overhead_count
                    result[key]["payload"] += payload_count
                else:
                    result[key] = {
                        "overhead": overhead_count,
                        "payload": payload_count
                    }

        keys = sorted(result.keys())
        payloads = [result[k]['payload'] / 30 for k in keys]
        overheads = [result[k]['overhead'] / 30 for k in keys]
        x_positions = range(len(keys))

        ratios = [
            (overhead / payload) if payload != 0 else 0
            for overhead, payload in zip(overheads, payloads)
        ]
        slope, intercept, _, _, _ = linregress(x_positions, ratios)

        _, ax1 = plt.subplots(figsize=(15, 6))
        
        ax1.bar(x_positions, payloads, label='payload', color='#3498db')
        ax1.bar(x_positions, overheads, bottom=payloads, label='overhead', color='#f39c12')
        ax1.set_xticks(x_positions)
        ax1.set_xticklabels(keys)
        ax1.set_xlabel('routers')
        ax1.set_ylabel('packet count')
        ax1.set_yticks(range(0, int(max(payloads) + max(overheads)) + 5, 5))
        ax1.set_title(f'packet overhead vs routers for single transmission: {today_date}, 1-10 routers in series (30 measurement averaged), st-andrews cs network')
        
        ax2 = ax1.twinx()
        ax2.set_ylabel('packet overhead index')
        ax2.plot( x_positions, ratios, label=f'packet overhead index: Y = {slope:.0f}X + {intercept:.0f}', color='green', marker='o')
        
        lines1, labels1 = ax1.get_legend_handles_labels()
        lines2, labels2 = ax2.get_legend_handles_labels()
        ax1.legend(lines1 + lines2, labels1 + labels2, loc='upper left')

        plt.subplots_adjust(left=0.05, right=0.96, top=0.94, bottom=0.09)
        plt.savefig("plot_packet_overhead_single.png")
        plt.close()


def packet_overhead_flow():
    with open("packet_overhead_flow.txt", "r") as file:

        result = {}

        for line in file:
            metric = line.strip()

            if "PCB" in metric:

                metric_split = metric.split(";")
                key = int(metric_split[4])
                pcb_string = metric_split[5]
                pcb = json.loads(pcb_string)

                overhead_count = 0
                payload_count = 0
                overhead_count += int(pcb.get("nd_solicitation_jcmp_tx"))
                overhead_count += int(pcb.get("nd_advertisement_jcmp_tx"))
                overhead_count += int(pcb.get("dns_fqdn_query_jcmp_tx"))
                overhead_count += int(pcb.get("dns_fqdn_response_jcmp_tx"))
                overhead_count += int(pcb.get("dns_ilv_query_jcmp_tx"))
                overhead_count += int(pcb.get("dns_ilv_response_jcmp_tx"))
                overhead_count += int(pcb.get("router_request_jcmp_tx"))
                overhead_count += int(pcb.get("router_response_jcmp_tx"))
                payload_count += int(pcb.get("data_request_tx"))

                if key in result:
                    result[key]["overhead"] += overhead_count
                    result[key]["payload"] += payload_count
                else:
                    result[key] = {
                        "overhead": overhead_count,
                        "payload": payload_count
                    }

        keys = sorted(result.keys())
        payloads = [result[k]['payload'] / 30 for k in keys]
        overheads = [result[k]['overhead'] / 30 for k in keys]
        x_positions = range(len(keys))

        ratios = [
            (overhead / payload) if payload != 0 else 0
            for overhead, payload in zip(overheads, payloads)
        ]
        _, ax1 = plt.subplots(figsize=(15, 6))
        
        ax1.bar(x_positions, payloads, label='payload', color='#3498db')
        ax1.bar(x_positions, overheads, bottom=payloads, label='overhead', color='#f39c12')
        ax1.set_xticks(x_positions)
        ax1.set_xticklabels(keys)
        ax1.set_xlabel('routers')
        ax1.set_ylabel('packet count')
        ax1.set_title(f'packet overhead vs routers for 10Mbps flow: {today_date}, 1-10 routers in series (30 measurement averaged), st-andrews cs network')
        
        ax2 = ax1.twinx()
        ax2.set_ylabel(f'packet overhead index')
        ax2.plot(x_positions, ratios, label=f'packet overhead index', color='green', marker='o')
        
        lines1, labels1 = ax1.get_legend_handles_labels()
        lines2, labels2 = ax2.get_legend_handles_labels()
        ax1.legend(lines1 + lines2, labels1 + labels2, loc='upper left')

        plt.subplots_adjust(left=0.05, right=0.95, top=0.94, bottom=0.09)
        plt.savefig("plot_packet_overhead_flow.png")
        plt.close()


def packet_throughput():

    file_names = ["throughput_single_machine", "throughput_multi_machine"]
    for file_name in file_names:

        with open(f"packet_{file_name}.txt", "r") as file:

            result = {}

            for line in file:
                metric = line.strip()

                if "PCB" in metric:

                    metric_split = metric.split(";")
                    key = int(metric_split[4])
                    pcb_string = metric_split[5]
                    pcb = json.loads(pcb_string)

                    packet_sent = 0
                    packet_received = 0
                    packet_sent += int(pcb.get("data_request_tx"))
                    packet_received += int(pcb.get("data_request_rx"))

                    bytes_sent = packet_sent * 1412 * 8
                    bytes_received = packet_received * 1412 * 8

                    if key in result:
                        result[key]["sent"] += bytes_sent
                        result[key]["received"] += bytes_received
                    else:
                        result[key] = {
                            "sent": bytes_sent,
                            "received": bytes_received
                        }

            keys = list(result.keys())
            sent = [ (result[k]['sent'] / 30) / 1_000_000 for k in keys]
            received = [ (result[k]['received'] / 30) / 1_000_000 for k in keys]

            fig, ax = plt.subplots(figsize=(15, 5))
            plt.subplots_adjust(left=0.05, right=0.98)
            ax.fill_between(keys, sent, color='#f39c12', alpha=0.3, label='mbps sent')
            ax.plot(keys, sent, 'o', color='#f39c12', markersize=8)
            ax.set_xlabel('routers')
            ax.set_ylabel('throughput (mbps)')
            ax.legend(loc='lower left', fontsize=14)
            ax.set_xticks(range(min(keys), max(keys) + 1))
            if file_name == "throughput_single_machine":
                ax.set_yticks(range(0, int(max(sent)) + 201, 200))
            elif file_name == "throughput_multi_machine":
                ax.set_yticks(range(0, int(max(sent)) + 51, 50))
            ax.grid(True, linestyle='--', alpha=0.6)
            fig.suptitle(f'sent throughput vs routers: {today_date}, 0-10 routers in series (30-second flows), st andrews cs network')
            plt.savefig(f"plot_{file_name}_sent.png")
            plt.close(fig)

            fig, ax = plt.subplots(figsize=(15, 5))
            plt.subplots_adjust(left=0.05, right=0.98)
            ax.fill_between(keys, received, color='#3498db', alpha=0.3, label='mbps received')
            ax.plot(keys, received, 'o', color='#3498db', markersize=8)
            ax.set_xlabel('routers')
            ax.set_ylabel('throughput (mbps)')
            ax.legend(loc='lower left', fontsize=14)
            ax.set_xticks(range(min(keys), max(keys) + 1))
            if file_name == "throughput_single_machine":
                ax.set_yticks(range(0, int(max(sent)) + 201, 200))
            elif file_name == "throughput_multi_machine":
                ax.set_yticks(range(0, int(max(sent)) + 51, 50))
            ax.grid(True, linestyle='--', alpha=0.6)
            fig.suptitle(f'received throughput vs routers: {today_date}, 0-10 routers in series (30-second flows), st andrews cs network')
            plt.savefig(f"plot_{file_name}_received.png")
            plt.close(fig)


def packet_errors():
    break_numbers = []
    futex_errors = []
    recvfrom_errors = []

    with open("system_call_logs.txt", 'r') as file:

        break_number = -1
        total_futex_error = 0
        total_recvfrom_error = 0

        for line in file:
            line = line.strip()

            if "BREAK" in line:
                break_number += 1
                break_numbers.append(break_number)
                futex_errors.append(total_futex_error)
                recvfrom_errors.append(total_recvfrom_error)
                continue
           
            if "futex" in line:
                try:
                    futex_error = int(line.split()[4])
                    total_futex_error += futex_error
                    continue
                except:
                    continue

            if "recvfrom" in line:
                try:
                    recvfrom_error = int(line.split()[4])
                    total_recvfrom_error += recvfrom_error
                    continue
                except:
                    continue

    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(20, 10))

    ax1.plot(break_numbers[:6], futex_errors[:6], marker='o', label='futex errors', linewidth=2, color='orange')
    ax1.set_xlabel('routers', fontsize=14)
    ax1.set_ylabel('error count', fontsize=14)
    ax1.set_title('futex errors vs routers, 0-5 routers in series, st andrews cs network')
    ax1.grid(True, which='both', linestyle='--', linewidth=0.5)
    ax1.legend(fontsize=12)

    ax2.plot(break_numbers[:6], recvfrom_errors[:6], marker='o', label='recvfrom errors', linewidth=2, color='orange')
    ax2.set_xlabel('routers', fontsize=14)
    ax2.set_ylabel('error count', fontsize=14)
    ax2.set_title('recvfrom errors vs routers, 0-5 routers in series, st andrews cs network')
    ax2.grid(True, which='both', linestyle='--', linewidth=0.5)
    ax2.legend(fontsize=12)

    fig.tight_layout()
    fig.savefig("plot_system_call.png")
    plt.close(fig)


def packet_latency_rtt():

    file_names = ["packet_latency_rtt_single_machine", "packet_latency_rtt_multi_machine"]
    for file_name in file_names:

        with open(f"{file_name}.txt", "r") as file:

            result = {}
            first = {0: True, 1: True, 2: True, 3: True,  4: True, 5: True, 6: True, 7: True, 8: True, 9: True, 10: True}

            for line in file:
                metric = line.strip()

                if "METRIC" in metric:

                    metric_split = metric.split(";")
                    key = int(metric_split[4])
                    rtt_reading = int(metric_split[5])

                    if first[key] == True:
                        first[key] = False
                        continue

                    if key in result:
                        result[key].append(rtt_reading)
                    else:
                        result[key] = [rtt_reading]

            flierprops = dict(marker='.', markerfacecolor='blue', markersize=2)
            medianprops = dict(linestyle='-', color='orange', linewidth=2)
            meanprops = dict(linestyle='--', linewidth=2, color='g')

            keys = list(result.keys())
            rtt = [[value for value in result[key]] for key in keys]
            mean_latency = [sum(values) / len(values) / 2 for values in rtt]

            x = np.arange(1, len(keys) + 1)
            a, b = np.polyfit(x, mean_latency, 1)

            plt.figure(figsize=(12, 8))
            plt.boxplot(rtt, tick_labels=keys, flierprops=flierprops, medianprops=medianprops, meanprops=meanprops, showmeans=True, meanline=True)
            plt.title(f'rtt vs routers: {today_date}, 0-10 routers in series (2000 measurements, 5th/95th whiskers), st-andrews cs network', fontsize=10)
            plt.xlabel('routers')
            plt.ylabel('round-trip-time (μs)')
            median_line = plt.Line2D([0], [1], color='orange', linestyle='-', linewidth=2)
            mean_line = plt.Line2D([0], [1], color='green', linestyle='--', linewidth=2)
            plt.legend([median_line, mean_line], ['median', 'mean'], loc='upper right')
            plt.subplots_adjust(left=0.07, right=0.97, top=0.95, bottom=0.08)
            plt.grid(axis='y', linestyle='-', alpha=0.7)
            plt.savefig(f"plot_{file_name}.png")
            plt.close()

            plt.figure(figsize=(12, 3))
            plt.scatter(x, mean_latency, color='green', label='mean latency (rtt/2)', zorder=3)
            plt.plot(x, a * x + b, color='orange', linestyle='-', linewidth=2)
            plt.title(f'latency vs routers: {today_date}, 0-10 routers in series (2000 measurements mean), st-andrews cs network', fontsize=10)
            plt.xlabel('routers')
            plt.ylabel('latency (μs)')
            mean_latency_line = plt.Line2D([0], [1], color='orange', linestyle='-', linewidth=2)
            plt.legend([mean_latency_line], [f'mean latency (best fit), y = {a:.0f}x + {b:.0f}'], loc='lower right')
            plt.subplots_adjust(left=0.07, right=0.97, top=0.92, bottom=0.12)
            plt.grid(axis='y', linestyle='-', alpha=0.7)
            plt.savefig(f"plot_{file_name}_latency.png")
            plt.close()


path_discovery_convergence()
packet_overhead_single()
packet_overhead_flow()
packet_throughput()
packet_errors()
packet_latency_rtt()

from datetime import datetime
import json
import matplotlib.pyplot as plt
import numpy as np

today_date = datetime.today().strftime('%Y-%m-%d')

def extract_data():
    measurements = {}

    with open("sensor_application.txt", "r") as file:

        for line in file.readlines():   
            line = line.strip()

            if "TOP" in line:
                line_split = line.split(";")

                topology = int(line_split[1])
                sink = str(line_split[3])
                device = int(line_split[5])
                temperature = float(line_split[7])
                humidity = float(line_split[9])
                soilmois = float(line_split[11])

                if topology not in measurements:
                    measurements[topology] = {}

                if sink not in measurements[topology]:
                    measurements[topology][sink] = 0

                if "devices" not in measurements[topology]:
                    measurements[topology]["devices"] = {}

                if device not in measurements[topology]["devices"]:
                    measurements[topology]["devices"][device] = {}

                if "temperature" not in measurements[topology]["devices"][device]:
                    measurements[topology]["devices"][device]["temperature"] = []

                if "humidity" not in measurements[topology]["devices"][device]:
                    measurements[topology]["devices"][device]["humidity"] = []

                if "soilmois" not in measurements[topology]["devices"][device]:
                    measurements[topology]["devices"][device]["soilmois"] = []

                measurements[topology][sink] += 1
                measurements[topology]["devices"][device]["temperature"].append(temperature)
                measurements[topology]["devices"][device]["humidity"].append(humidity)
                measurements[topology]["devices"][device]["soilmois"].append(soilmois)

            if "PCB" in line:
                line_split = line.split(";")

                topology = int(line_split[4])
                pcb_string = line_split[5]
                pcb = json.loads(pcb_string)

                if topology not in measurements:
                    measurements[topology] = {}

                if "forwarded" not in measurements[topology]:
                    measurements[topology]["forwarded"] = 0

                forward_count = int(pcb.get("data_request_forward_tx"))
                measurements[topology]["forwarded"] += forward_count

    return measurements

def plot_measurements(topology_measurements):
    all_devices = sorted(topology_measurements.keys())
    cols = 5
    rows = 2
    
    fig, axes = plt.subplots(rows, cols, figsize=(20, 4), sharex=True)
    axes = axes.flatten()

    all_handles = []
    all_labels = []

    for i, device in enumerate(all_devices):
        ax = axes[i]
        data = topology_measurements[device]
        x = range(len(data['temperature']))

        temp_line, = ax.plot(x, data['temperature'], label='temperature', color='r', linestyle='-')
        if i == 0:
            all_handles.append(temp_line)
            all_labels.append('temperature')

        if i % cols == 0:
            ax.set_ylabel("temperature (°C)", color='r')
        ax.tick_params(axis='y', labelcolor='r')

        ax2 = ax.twinx()
        hum_line, = ax2.plot(x, data['humidity'], label='humidity', color='b', linestyle='-')
        soil_line, = ax2.plot(x, data['soilmois'], label='soil moisture', color='g', linestyle='-')
        if i == 0:
            all_handles.extend([hum_line, soil_line])
            all_labels.extend(['humidity', 'soil moisture'])

        if (i + 1) % cols == 0 or i == len(all_devices) - 1:
            ax2.set_ylabel("percentage (%)", color='b')
        ax2.tick_params(axis='y', labelcolor='b')

        ax.set_title(f"device {device} measurements")

    for ax in axes[-cols:]:
        ax.set_xlabel("time index")

    fig.legend(all_handles, all_labels, loc='lower center', ncol=3, bbox_to_anchor=(0.5, -0.02))
    plt.tight_layout(rect=[0, 0.03, 1, 1])
    plt.savefig("plot_sensor_measurements.png")
    plt.close()

def plot_stretch(data, expected):

    actual = []

    for key in data:
        entry = [data[key]["node1"]]
        entry.append(data[key]["node2"])
        entry.append(data[key]["forwarded"])
        actual.append(entry)

    print(actual)
    print(expected)

    np_actual = np.array(actual)
    np_expected = np.array(expected)
    index = np_actual / np_expected
    x_values = data.keys()

    y_sink1_values = index[:, 0]
    plt.figure(figsize=(10, 1.5))
    plt.plot(x_values, y_sink1_values, marker='o', linestyle='-', color='red', label=f'sink1 stretch')
    plt.title(f'sink1 stretch vs topology: {today_date}, 2 sinks, 10 sensors (20 measurements summed), st-andrews cs network')
    plt.xlabel('topology')
    plt.ylabel('sink1 stretch')
    plt.ylim(0, 2)
    plt.legend()
    plt.grid(True)
    plt.subplots_adjust(top=0.8, bottom=0.2, left=0.1, right=0.9)
    plt.savefig(f'plot_stretch_sink1.png') 
    plt.close()

    y_sink2_values = index[:, 1]
    plt.figure(figsize=(10, 1.5))
    plt.plot(x_values, y_sink2_values, marker='o', linestyle='-', color='red', label=f'sink2 stretch')
    plt.title(f'sink2 stretch vs topology: {today_date}, 2 sinks, 10 sensors (20 measurements summed), st-andrews cs network')
    plt.xlabel('topology')
    plt.ylabel('sink2 stretch')
    plt.ylim(0, 2)
    plt.legend()
    plt.grid(True)
    plt.subplots_adjust(top=0.8, bottom=0.2, left=0.1, right=0.9)
    plt.savefig(f'plot_stretch_sink2.png') 
    plt.close()

    y_path_values = index[:, 2]
    plt.figure(figsize=(10, 1.5))
    plt.plot(x_values, y_path_values, marker='o', linestyle='-', color='red', label=f'path stretch')
    plt.title(f'path stretch vs topology: {today_date}, 2 sinks, 10 sensors (20 measurements summed), st-andrews cs network')
    plt.xlabel('topology')
    plt.ylabel('path stretch')
    plt.ylim(0, 2)
    plt.legend()
    plt.grid(True)
    plt.subplots_adjust(top=0.8, bottom=0.2, left=0.1, right=0.9)
    plt.savefig(f'plot_stretch_path.png') 
    plt.close()


def main():
    data = extract_data()

    # expected sink1 sink2 packet_forward_count
    # for each topology
    expected = [
        [100, 100, 360],
        [100, 100, 320],
        [100, 100, 320],
        [120, 80, 280],
        [100, 100, 240],
        [160, 40, 280],
        [100, 100, 320],
        [100, 100, 280],
        [100, 100, 320]
    ]

    # measurements
    plot_measurements(data[1]["devices"])
    plot_stretch(data, expected)


main()

